use chrono::{ DateTime, Utc};
use arrow::array::{Array, StringArray, Int64Array, TimestampMillisecondArray};
use arrow_array::RecordBatch;
use parquet::arrow::arrow_reader::{ParquetRecordBatchReaderBuilder, ParquetRecordBatchReader};
use serde::{Serialize, Deserialize};
pub mod configuration;
use configuration::get_configuration;
use std::{collections::HashMap, fs::File};
use auditor::domain::Record;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub amount: i32,
    pub scores: Vec<Score>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordData {
    pub record_id: String,
    pub meta: HashMap<String, serde_json::Value>,
    pub components: Vec<Component>,
    pub start_time: DateTime<Utc>,
    pub stop_time: DateTime<Utc>,
    pub runtime: i64,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {

            let configuration = get_configuration().expect("Failed to read configuration.");
        
            let file_path = &configuration.file_path;

            let database_url = std::env::var("DATABASE_URL")
    .unwrap_or_else(|_| configuration.to_url());

            println!("{database_url}");
            let file = File::open(&file_path)?;
            let arrow_reader = ParquetRecordBatchReaderBuilder::try_new(file)?;
            
            let parquet_metadata = arrow_reader.metadata();

            let num_rows = parquet_metadata.file_metadata().num_rows();

            println!("Number of records in '{:?}': {}", &file_path, num_rows);
                      
        let _ = restore_to_db(arrow_reader.build()?, database_url).await;
    

    Ok(())

}


fn arrow_batch_to_records(batch: RecordBatch) -> anyhow::Result<Vec<Record>> {
        let mut records = Vec::new();
        let num_rows = batch.num_rows();
        
        let record_id_array = batch.column(0).as_any().downcast_ref::<StringArray>().unwrap();
        let meta_array = batch.column(1).as_any().downcast_ref::<StringArray>().unwrap();
        let components_array = batch.column(2).as_any().downcast_ref::<StringArray>().unwrap();
        let start_time_array = batch.column(3).as_any().downcast_ref::<TimestampMillisecondArray>().unwrap();
        let stop_time_array = batch.column(4).as_any().downcast_ref::<TimestampMillisecondArray>().unwrap();
        let runtime_array = batch.column(5).as_any().downcast_ref::<Int64Array>().unwrap();


        for i in 0..num_rows {
            let record = Record {
                record_id: record_id_array.value(i).to_string(),
                meta: serde_json::from_str(meta_array.value(i))?,
                components: serde_json::from_str(components_array.value(i))?,
                start_time: Some(DateTime::from_timestamp_millis(start_time_array.value(i))
                    .unwrap()
                    .with_timezone(&Utc)),
                stop_time: Some(DateTime::from_timestamp_millis(stop_time_array.value(i))
                    .unwrap()
                    .with_timezone(&Utc)),
                runtime: Some(runtime_array.value(i)),
            };

             records.push(record);
        }

        Ok(records)
    }

use sqlx::PgPool;
use serde_json::Value;

async fn restore_to_db(arrow_reader: ParquetRecordBatchReader, database_url: String) -> anyhow::Result<()> {



    
    // Create a connection pool
    let pool = PgPool::connect(&database_url).await?;
    
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => return Err(e.into()),
    };

    println!("Your parquet file is being processed back to AUDITOR db. Please wait ---");
        
        for batch in arrow_reader {
        
            let batch = batch?;
            
            let records = arrow_batch_to_records(batch.clone())?;
    
        let record_ids: Vec<_> = records
        .iter()
        .map(|r| r.record_id.clone())
        .collect();
    let start_times: Vec<_> = records.iter().map(|r| r.start_time.unwrap()).collect();
    let stop_times: Vec<_> = records.iter().map(|r| r.stop_time.unwrap()).collect();
    let runtimes: Vec<_> = records
        .iter()
        .map(|r| r.runtime.unwrap())
        .collect();
    let updated_at_vec: Vec<_> = std::iter::repeat(Utc::now()).take(records.len()).collect();

    let meta_values: Vec<Value> = records
        .iter()
        .map(|r| serde_json::to_value(&r.meta).unwrap_or(serde_json::Value::Null))
        .collect();
    let component_values: Vec<Value> = records
        .iter()
        .map(|r| serde_json::to_value(&r.components).unwrap_or(serde_json::Value::Null))
        .collect();

    sqlx::query!(
        r#"
        INSERT INTO auditor_accounting (
            record_id, start_time, stop_time, meta, components, runtime, updated_at
        )
        SELECT * FROM UNNEST($1::text[], $2::timestamptz[], $3::timestamptz[], $4::jsonb[], $5::jsonb[],  $6::bigint[], $7::timestamptz[])
        RETURNING id;
        "#,
        &record_ids[..],
        &start_times[..],
        &stop_times[..],
        &meta_values[..],
        &component_values[..],
        &runtimes[..],
        &updated_at_vec[..],
    )
    .fetch_all(&mut *transaction)
    .await.unwrap();
            
    }

    if let Err(e) = transaction.commit().await {
        return Err(e.into());
    } else {
        println!("Parquet file successfully loaded to AUDITOR");
    }

    

        return Ok(())
}
