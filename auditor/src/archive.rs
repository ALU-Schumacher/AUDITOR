use crate::configuration::ArchivalConfig;
use crate::configuration::CompressionType;
use crate::domain::Record;
use anyhow;
use arrow::array::{Int64Array, StringArray, TimestampMillisecondArray};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow_array::RecordBatch;
use chrono::{DateTime, Datelike, Utc};
use chrono::{Months, TimeZone};
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::basic::{Compression, GzipLevel};
use parquet::file::properties::WriterProperties;
use sqlx::FromRow;
use sqlx::{PgPool, Row};
use std::fs::File;
use std::ops::Add;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

#[derive(Debug, Clone)]
pub struct ArchiveService {
    pub pool: PgPool,
    pub config: ArchivalConfig,
}

impl ArchiveService {
    pub fn new(pool: PgPool, config: ArchivalConfig) -> Self {
        Self { pool, config }
    }

    pub async fn start_scheduler(&self) -> anyhow::Result<()> {
        let scheduler = JobScheduler::new().await?;

        let pool = self.pool.clone();
        let config = self.config.clone();

        {
            let pool = pool.clone();
            let config = config.clone();
            println!("Archival process is running");
            match Self::archive_old_records(pool, config).await {
                Ok(_) => println!("successfully archived records"),
                Err(e) => println!("Archival process failed. Check the logs for more info {e}"),
            }
        }

        let job = Job::new_async(
            config.cron_schedule.clone().as_str(),
            move |_uuid, _lock| {
                let pool = pool.clone();
                let config = config.clone();
                Box::pin(async move {
                    info!("Started scheduled archival process");
                    match Self::archive_old_records(pool, config).await {
                        Ok(_) => println!("Successfully archived records"),
                        Err(e) => {
                            println!("Archival process failed. Check the logs for more info {e}",)
                        }
                    }
                })
            },
        )?;

        scheduler.add(job).await?;
        scheduler.start().await?;

        Ok(())
    }

    async fn archive_old_records(pool: PgPool, config: ArchivalConfig) -> anyhow::Result<()> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("record_id", DataType::Utf8, false),
            Field::new("meta", DataType::Utf8, false),
            Field::new("components", DataType::Utf8, false),
            Field::new(
                "start_time",
                DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())),
                false,
            ),
            Field::new(
                "stop_time",
                DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())),
                false,
            ),
            Field::new("runtime", DataType::Int64, false),
        ]));

        let archive_period = Months::new(u32::try_from(config.archive_older_than_months)?);
        let current_timestamp = Utc::now();

        let cutoff_timestamp = current_timestamp
            .checked_sub_months(archive_period)
            .expect("Error while constructing cutoff_timestamp for archiving");

        let cutoff_month = cutoff_timestamp.month();
        let cutoff_year = cutoff_timestamp.year();

        let cutoff_timestamp_lower_bound = Utc
            .with_ymd_and_hms(cutoff_year, cutoff_month, 1, 0, 0, 0)
            .unwrap();

        #[derive(Debug, FromRow)]
        struct StopTime {
            stop_time: DateTime<Utc>,
        }
        let oldest_record_timestamp_in_db = sqlx::query_as::<_, StopTime>(
            r"SELECT stop_time FROM auditor_accounting ORDER BY stop_time ASC limit 1;",
        )
        .fetch_optional(&pool)
        .await?;

        let earliest_timestamp = match oldest_record_timestamp_in_db {
            Some(ref record_timestamp) => {
                println!("Oldest record timestamp stop_time: {record_timestamp:?}");
                record_timestamp.stop_time
            }
            None => {
                println!(
                    "The database is empty. No records found in auditor_accounting for archival."
                );
                return Ok(());
            }
        };

        let mut year = earliest_timestamp.year();
        let mut month = earliest_timestamp.month();
        let mut month_to_be_archived_lower_bound =
            Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();
        let archive_interval = Months::new(1);
        let mut month_to_be_archived_upper_bound = month_to_be_archived_lower_bound
            .checked_add_months(archive_interval)
            .unwrap();

        while month_to_be_archived_lower_bound < cutoff_timestamp_lower_bound
            && month_to_be_archived_upper_bound <= cutoff_timestamp_lower_bound
        {
            year = month_to_be_archived_lower_bound.year();
            month = month_to_be_archived_lower_bound.month();

            let archive_filename = format!(
                "{}_{}_{}.parquet",
                config.archive_file_prefix, &year, &month,
            );

            let a = month_to_be_archived_lower_bound.to_rfc3339();
            let b = month_to_be_archived_upper_bound.to_rfc3339();

            let archive_dir = Path::new(&config.archive_path);

            if !archive_dir.exists() {
                println!(
                    "Directory does not exist. Creating new directory at {archive_dir:?} (as specified in config.archive_path)"
                );
                if let Err(e) = std::fs::create_dir_all(archive_dir) {
                    eprintln!("Failed to create directory: {e}");
                }
            }

            let path = archive_dir.join(&archive_filename);

            let mut writer: Option<ArrowWriter<File>> = None;
            let mut total_archived = 0i64;
            let mut offset = 0i64;

            loop {
                let records_sql = sqlx::query(
                        r"SELECT record_id,
                                  meta,
                                  components,
                                  start_time,
                                  stop_time,
                                  runtime
                           FROM auditor_accounting WHERE stop_time >= $1::timestamptz AND stop_time < $2::timestamptz ORDER BY id
                                LIMIT $3 OFFSET $4"
                    ).bind(&a).bind(&b).bind(1000000)
                            .bind(offset).fetch_all(&pool).await?;

                if records_sql.is_empty() {
                    break;
                }

                if writer.is_none() {
                    let file = File::create(&path)?;

                    let props = match config.compression_type {
                        CompressionType::Gzip => WriterProperties::builder()
                            .set_compression(Compression::GZIP(GzipLevel::default()))
                            .build(),
                        CompressionType::Snappy => WriterProperties::builder()
                            .set_compression(Compression::SNAPPY)
                            .build(),
                    };

                    writer = Some(ArrowWriter::try_new(file, schema.clone(), Some(props))?);
                }

                let batch_count = records_sql.len() as i64;

                let records: Vec<Record> = records_sql
                    .iter()
                    .map(|row| Record {
                        record_id: row.try_get("record_id").unwrap(),
                        meta: row
                            .try_get("meta")
                            .ok()
                            .and_then(|value| serde_json::from_value(value).ok()),
                        components: row
                            .try_get("components")
                            .ok()
                            .and_then(|value| serde_json::from_value(value).ok()),
                        start_time: row.try_get("start_time").ok().unwrap_or(None),
                        stop_time: row.try_get("stop_time").ok().unwrap_or(None),
                        runtime: row.try_get("runtime").ok().unwrap_or(None),
                    })
                    .collect();

                // Convert data to Arrow arrays
                let record_ids: Vec<String> = records.iter().map(|r| r.record_id.clone()).collect();
                let metas: Vec<String> = records
                    .iter()
                    .map(|r| serde_json::to_string(&r.meta).unwrap())
                    .collect();
                let components: Vec<String> = records
                    .iter()
                    .map(|r| serde_json::to_string(&r.components).unwrap())
                    .collect();
                let start_times: Vec<i64> = records
                    .iter()
                    .map(|r| r.start_time.expect("None value").timestamp_millis())
                    .collect();
                let stop_times: Vec<i64> = records
                    .iter()
                    .map(|r| r.stop_time.expect("None value").timestamp_millis())
                    .collect();
                let runtimes: Vec<i64> = records.iter().map(|r| r.runtime.expect("None")).collect();

                let batch = RecordBatch::try_new(
                    schema.clone(),
                    vec![
                        Arc::new(StringArray::from(record_ids)),
                        Arc::new(StringArray::from(metas)),
                        Arc::new(StringArray::from(components)),
                        Arc::new(TimestampMillisecondArray::from(start_times).with_timezone("UTC")),
                        Arc::new(TimestampMillisecondArray::from(stop_times).with_timezone("UTC")),
                        Arc::new(Int64Array::from(runtimes)),
                    ],
                )?;

                writer
                    .as_mut()
                    .expect("Writer should be initialized")
                    .write(&batch)?;

                total_archived += batch_count;
                offset += 1000000;

                info!(
                    "Archived batch of {} records to {}",
                    batch_count, archive_filename
                );

                writer
                    .as_mut()
                    .expect("Writer should be initialized")
                    .flush()?;
            }

            if let Some(w) = writer {
                w.close()?;

                info!("Total records archived: {}", total_archived);

                let validated_data = data_validation(&path, pool.clone(), &a, &b).await;

                match validated_data {
                    Ok(record_count) => {
                        println!("Validation is successful for {:?}", &path);

                        deletion_from_db(&a, &b, record_count, pool.clone()).await?;
                    }
                    Err(e) => {
                        println!("{e}. Aborting deletion of data for month/year ");
                    }
                }
            } else {
                info!(
                    "No records found for period {} to {}, skipping file creation",
                    a, b
                );
            }

            month_to_be_archived_lower_bound = month_to_be_archived_lower_bound
                .checked_add_months(archive_interval)
                .unwrap();

            month_to_be_archived_upper_bound = month_to_be_archived_upper_bound
                .checked_add_months(archive_interval)
                .unwrap();
        }

        Ok(())
    }
}

async fn deletion_from_db(
    a: &String,
    b: &String,
    record_count: i64,
    pool: PgPool,
) -> anyhow::Result<()> {
    let mut delete_count = 0i64;

    loop {
        let result = sqlx::query(
            r"WITH to_delete AS (
    SELECT *
    FROM auditor_accounting
    WHERE stop_time >= $1::timestamptz
      AND stop_time <  $2::timestamptz
    LIMIT 10
)
DELETE FROM auditor_accounting
USING to_delete
WHERE auditor_accounting.id = to_delete.id;",
        )
        .bind(a)
        .bind(b)
        .execute(&pool)
        .await?
        .rows_affected();

        delete_count += i64::try_from(result)?;

        if result == 0 && delete_count == record_count {
            println!("Record deletion is successful. count --> {delete_count}");
            break;
        }

        if delete_count > record_count {
            println!(
                "Something went wrong while deletion. Please check the data for the month from this timestamp --> {a} "
            );
            break;
        }
    }

    Ok(())
}

async fn data_validation(
    parquet_file_path: &PathBuf,
    pool: PgPool,
    a: &String,
    b: &String,
) -> anyhow::Result<i64> {
    println!("checking file path {:?}", &parquet_file_path);
    let file = File::open(parquet_file_path)?;
    let arrow_reader = ParquetRecordBatchReaderBuilder::try_new(file)?;

    let parquet_metadata = arrow_reader.metadata();

    let num_rows = parquet_metadata.file_metadata().num_rows();

    println!("Number of rows in '{:?}': {}", &parquet_file_path, num_rows);

    let mut record_count: usize = 0;

    for batch in arrow_reader.build()? {
        record_count = batch?.num_rows().add(record_count);
    }

    println!("record count --> {record_count}");

    let converted_count: i64 = record_count.try_into().expect("Conversion failed");

    let row = sqlx::query("SELECT COUNT(*) AS count FROM auditor_accounting WHERE stop_time > $1::timestamptz and stop_time <= $2::timestamptz")
            .bind(a)
            .bind(b)
            .fetch_one(&pool)
            .await?;

    let count: i64 = row.get("count");

    println!("count_query_from_db --> {:?}", &count);

    if converted_count == count {
        Ok(count)
    } else {
        Err(anyhow::anyhow!(
            "The record count in {:?} --> {} doesn't match the database {}. Suspending archival service",
            parquet_file_path,
            converted_count,
            count
        ))
    }
}
