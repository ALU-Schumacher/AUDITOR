// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::str::FromStr;

use auditor::domain::RecordAdd;
use chrono::{offset::Local, offset::TimeZone, DateTime, NaiveDateTime};
use color_eyre::eyre::{eyre, Result};
use sqlx::{sqlite::SqliteJournalMode, SqlitePool};

use crate::CONFIG;

#[derive(Clone)]
pub(crate) struct Database {
    db_pool: SqlitePool,
}

impl Database {
    #[tracing::instrument(name = "Initializing sqlite database connection", level = "debug")]
    pub(crate) async fn new<S: AsRef<str> + std::fmt::Debug>(path: S) -> Result<Database> {
        let db_pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::from_str(path.as_ref())?
                .journal_mode(SqliteJournalMode::Wal)
                .create_if_missing(true),
        )
        .await?;
        tracing::debug!("Migrating database");
        sqlx::migrate!().run(&db_pool).await?;
        Ok(Database { db_pool })
    }

    #[tracing::instrument(name = "Inserting record into database", level = "debug", skip(self))]
    pub(crate) async fn insert(&self, record: RecordAdd) -> Result<()> {
        let record_id = record.record_id.clone();
        let record = bincode::serialize(&record)?;
        sqlx::query!(
            r#"INSERT OR IGNORE INTO records (id, record) VALUES ($1, $2)"#,
            record_id,
            record
        )
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    #[tracing::instrument(name = "Deleting record from database", level = "debug", skip(self))]
    pub(crate) async fn delete(&self, record_id: String) -> Result<()> {
        sqlx::query!(r#"DELETE FROM records WHERE id=$1"#, record_id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    #[tracing::instrument(name = "Retrieving records from database", level = "debug", skip(self))]
    pub(crate) async fn get_records(&self) -> Result<Vec<(String, RecordAdd)>> {
        struct Row {
            id: String,
            record: Vec<u8>,
        }
        let records: Vec<Row> = sqlx::query_as!(Row, r#"SELECT id, record FROM records"#)
            .fetch_all(&self.db_pool)
            .await?;
        Ok(records
            .into_iter()
            .map(|Row { id, record }| (id, bincode::deserialize::<RecordAdd>(&record).unwrap()))
            .collect())
    }

    #[tracing::instrument(name = "Closing database connection", level = "info", skip(self))]
    pub(crate) async fn close(&self) {
        self.db_pool.close().await
    }

    #[tracing::instrument(
        name = "Retrieving last check datetime from database",
        level = "debug",
        skip(self)
    )]
    pub(crate) async fn get_lastcheck(&self) -> Result<(DateTime<Local>, String)> {
        struct Row {
            lastcheck: NaiveDateTime,
            jobid: String,
        }
        match sqlx::query_as!(Row, r#"SELECT lastcheck, jobid FROM lastcheck"#)
            .fetch_optional(&self.db_pool)
            .await?
        {
            Some(Row { lastcheck, jobid }) => {
                Ok((Local.from_local_datetime(&lastcheck).unwrap(), jobid))
            }
            None => {
                let datetime = CONFIG.earliest_datetime;
                tracing::info!(
                    "No last check date found in database. Assuming {}",
                    datetime.format("%FT%T")
                );
                Ok((datetime, String::new()))
            }
        }
    }

    #[tracing::instrument(
        name = "Setting last check datetime in database",
        level = "debug",
        skip(self)
    )]
    pub(crate) async fn set_lastcheck(
        &self,
        job_id: String,
        timestamp: DateTime<Local>,
    ) -> Result<()> {
        let mut transaction = match self.db_pool.begin().await {
            Ok(transaction) => transaction,
            Err(e) => return Err(eyre!("Error initializing transaction: {:?}", e)),
        };
        sqlx::query!(r#"DELETE FROM lastcheck"#)
            .execute(&mut transaction)
            .await?;
        sqlx::query!(
            r#"INSERT INTO lastcheck (lastcheck, jobid) VALUES ($1, $2)"#,
            timestamp,
            job_id
        )
        .execute(&mut transaction)
        .await?;
        if let Err(e) = transaction.commit().await {
            Err(eyre!("Error commiting transaction: {:?}", e))
        } else {
            Ok(())
        }
    }
}
