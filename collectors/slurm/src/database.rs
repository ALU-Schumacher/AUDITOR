// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::str::FromStr;

use auditor::domain::Record;
use color_eyre::eyre::Result;
use sqlx::{sqlite::SqliteJournalMode, SqlitePool};

pub(crate) struct Database {
    db_pool: SqlitePool,
}

impl Database {
    pub(crate) async fn new<S: AsRef<str>>(path: S) -> Result<Database> {
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

    pub(crate) async fn _insert(&self, _record: Record) -> Result<()> {
        // let record_id = record.record_id.clone();
        // let record = bincode::serialize(&record);
        // sqlx::query!(r#"INSERT INTO records (id, record) VALUES (record_id, record)"#)
        //     .execute(&self.db_pool)
        //     .await;
        Ok(())
    }

    #[tracing::instrument(name = "Closing database connection", level = "info", skip(self))]
    pub(crate) async fn close(&self) {
        self.db_pool.close().await
    }
}
