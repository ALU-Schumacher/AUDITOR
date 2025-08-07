// Copyright 2021-2024 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::path::Path;
use std::str::FromStr;

use auditor::domain::RecordAdd;

use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{QueryBuilder, Sqlite, SqlitePool, sqlite::SqliteJournalMode};

// See https://docs.rs/sqlx/latest/sqlx/struct.QueryBuilder.html#method.push_bind
const BULK_SIZE: usize = 16384;

fn is_path_valid(path: &Path) -> bool {
    path.to_str().is_some_and(|s| !s.is_empty()) && path.try_exists().is_ok()
}

/// Dummy struct to read out records
struct RecRow {
    blob: Vec<u8>,
}

impl From<&RecRow> for RecordAdd {
    fn from(v: &RecRow) -> Self {
        bincode::deserialize(&v.blob).expect("Should never fail on a record")
    }
}

/// A Wrapper around an SQLite database
///
#[derive(Clone)]
pub(crate) struct Database {
    db_pool: SqlitePool,
    maxretries: u16,
    interval: i64,
}

impl Database {
    /// Construct new database object
    #[tracing::instrument(name = "Initializing sqlite database connection", level = "debug")]
    pub(crate) async fn new(
        path: &Path,
        maxretries: u16,
        interval: i64,
    ) -> anyhow::Result<Database> {
        anyhow::ensure!(interval >= 0, "interval should be >= 0");
        // Sqlx gives us no error on empty paths...
        // Do some checks
        if !is_path_valid(path) {
            tracing::error!("Invalid path for database: {:?}", path);
            return Err(sqlx::Error::Io(std::io::Error::from(std::io::ErrorKind::Other)).into());
        };
        let db_pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(path)
                .journal_mode(SqliteJournalMode::Wal)
                .create_if_missing(true),
        )
        .await?;
        sqlx::migrate!().run(&db_pool).await?;
        Ok(Database {
            db_pool,
            maxretries,
            interval,
        })
    }

    #[allow(dead_code)]
    async fn in_memory(maxretries: u16, interval: i64) -> anyhow::Result<Database> {
        anyhow::ensure!(interval >= 0, "interval should be >= 0");
        let db_pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::from_str("sqlite://:memory:")?
                .journal_mode(SqliteJournalMode::Wal)
                .create_if_missing(true),
        )
        .await?;
        sqlx::migrate!().run(&db_pool).await?;
        Ok(Database {
            db_pool,
            maxretries,
            interval,
        })
    }

    //#[tracing::instrument(
    //    name = "Inserting record into database",
    //    level = "debug",
    //    skip(self, entry),
    //    fields(record_id = %entry.rid)
    //)]
    //pub(crate) async fn insert(&self, entry: &MergeEntry) -> Result<(), sqlx::Error> {
    //    sqlx::query!(
    //        r#"INSERT INTO mergequeue
    //            (record, rid, retry, updated, complete)
    //            VALUES ($1, $2, $3, $4, $5)"#,
    //        entry.record, entry.rid, entry.retry, entry.updated, entry.complete
    //    )
    //    .execute(&self.db_pool)
    //    .await?;
    //    Ok(())
    //}

    #[tracing::instrument(
        name = "Bulk inserting records into database",
        level = "debug",
        skip(self, entries)
    )]
    pub(crate) async fn insert_many(&self, entries: &[RecordAdd]) -> Result<(), sqlx::Error> {
        for chunk in entries.chunks(BULK_SIZE) {
            let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                "INSERT INTO mergequeue (record, rid, retry, updated, complete) ",
            );
            query_builder.push_values(chunk, |mut b, rec| {
                let blob = bincode::serialize(rec).expect("Should never fail on a record");
                b.push_bind(blob)
                    .push_bind(&rec.record_id)
                    .push_bind(0)
                    .push_bind(0)
                    .push_bind(false);
            });
            query_builder.build().execute(&self.db_pool).await?;
        }
        Ok(())
    }

    #[tracing::instrument(
        name = "Updating an incomplete record in database",
        level = "debug",
        skip(self, rec),
        fields(record_id = %rec.record_id)
    )]
    pub(crate) async fn replace_incomplete(&self, rec: &RecordAdd) -> Result<(), sqlx::Error> {
        let now = Utc::now().timestamp();
        let rid = rec.record_id.as_ref().to_owned();
        let blob = bincode::serialize(rec).expect("Should never fail on a record");
        sqlx::query!(
            r#"UPDATE mergequeue SET
                    record = $2,
                    retry = (SELECT retry FROM mergequeue WHERE rid=$1) + 1,
                    updated = $3
                WHERE rid=$1
                "#,
            rid,
            blob,
            now
        )
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    #[tracing::instrument(
        name = "Updating a complete record in database",
        level = "debug",
        skip(self, rec),
        fields(record_id = %rec.record_id)
    )]
    pub(crate) async fn replace_complete(&self, rec: &RecordAdd) -> Result<(), sqlx::Error> {
        let now = Utc::now().timestamp();
        let rid = rec.record_id.as_ref().to_owned();
        let blob = bincode::serialize(rec).expect("Should never fail on a record");
        sqlx::query!(
            r#"UPDATE mergequeue
                SET (record, updated, complete) = ($1, $2, $3)
                WHERE rid=$4
                "#,
            blob,
            now,
            true,
            rid
        )
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    #[tracing::instrument(name = "Deleting record from database", level = "debug", skip(self))]
    pub(crate) async fn delete(&self, rid: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"DELETE FROM mergequeue WHERE rid=$1"#, rid)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    #[tracing::instrument(
        name = "Getting mergeable records from database",
        level = "debug",
        skip(self)
    )]
    pub(crate) async fn get_mergequeue(&self) -> Result<Vec<RecordAdd>, sqlx::Error> {
        let time = Utc::now().timestamp() - self.interval;
        let rows = sqlx::query_as!(
            RecRow,
            r#"SELECT record as blob FROM mergequeue
            WHERE retry<=$1 AND updated<=$2 AND complete=FALSE"#,
            self.maxretries,
            time
        )
        .fetch_all(&self.db_pool)
        .await?;
        let recs = rows.iter().map(<&RecRow as Into<RecordAdd>>::into);
        Ok(recs.collect())
    }

    #[tracing::instrument(
        name = "Getting non-mergeable, incomplete records from database",
        level = "debug",
        skip(self)
    )]
    pub(crate) async fn get_incomplete(&self) -> Result<Vec<RecordAdd>, sqlx::Error> {
        let rows = sqlx::query_as!(
            RecRow,
            r#"SELECT record as blob FROM mergequeue
            WHERE retry>$1 AND complete=FALSE"#,
            self.maxretries
        )
        .fetch_all(&self.db_pool)
        .await?;
        let recs = rows.iter().map(<&RecRow as Into<RecordAdd>>::into);
        Ok(recs.collect())
    }

    #[tracing::instrument(
        name = "Getting complete records from database",
        level = "debug",
        skip(self)
    )]
    pub(crate) async fn get_complete(&self) -> Result<Vec<RecordAdd>, sqlx::Error> {
        let rows = sqlx::query_as!(
            RecRow,
            r#"SELECT record as blob FROM mergequeue WHERE complete=TRUE"#,
        )
        .fetch_all(&self.db_pool)
        .await?;
        let recs = rows.iter().map(<&RecRow as Into<RecordAdd>>::into);
        Ok(recs.collect())
    }

    #[tracing::instrument(name = "Setting last check time", level = "debug", skip(self))]
    pub(crate) async fn set_lastcheck(&self, time: DateTime<Utc>) -> Result<(), sqlx::Error> {
        let mut transaction = self.db_pool.begin().await?;
        sqlx::query!(r#"DELETE FROM lastcheck"#)
            .execute(&mut *transaction)
            .await?;
        sqlx::query!(r#"INSERT INTO lastcheck (time) VALUES ($1)"#, time)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await
    }

    #[tracing::instrument(name = "Getting last check time", level = "debug", skip(self))]
    pub(crate) async fn get_lastcheck(&self) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
        struct Row {
            time: NaiveDateTime,
        }
        let row = sqlx::query_as!(Row, r#"SELECT time FROM lastcheck"#)
            .fetch_optional(&self.db_pool)
            .await?;
        let time = row.map(|r| r.time.and_utc());
        Ok(time)
    }

    /// Closes the database connection
    #[allow(dead_code)]
    #[tracing::instrument(name = "Closing database connection", level = "debug", skip(self))]
    pub(crate) async fn close(&self) {
        self.db_pool.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auditor::domain::{Record, RecordTest};
    use fake::{Fake, Faker};

    fn record<T: TryFrom<RecordTest>>() -> T
    where
        <T as TryFrom<RecordTest>>::Error: std::fmt::Debug,
    {
        T::try_from(Faker.fake::<RecordTest>()).unwrap()
    }

    #[tokio::test]
    async fn insert_get() {
        let db = Database::in_memory(3, 0).await.unwrap();
        let rec: Vec<RecordAdd> = (0..10).map(|_| record()).collect();

        db.insert_many(&rec).await.unwrap();
        let res = db.get_mergequeue().await.unwrap();
        assert_eq!(db.get_incomplete().await.unwrap().len(), 0);
        assert_eq!(db.get_complete().await.unwrap().len(), 0);

        let mut rec: Vec<_> = rec.into_iter().map(Record::from).collect();
        let mut res: Vec<_> = res.into_iter().map(Record::from).collect();
        rec.sort();
        res.sort();
        assert_eq!(res, rec);
    }

    #[tokio::test]
    async fn insert_delete() {
        let db = Database::in_memory(3, 0).await.unwrap();
        let rec: Vec<RecordAdd> = (0..10).map(|_| record()).collect();
        let ids = rec.iter().map(|r| r.record_id.as_ref());

        db.insert_many(&rec).await.unwrap();
        assert_eq!(db.get_mergequeue().await.unwrap().len(), 10);

        for id in ids {
            db.delete(id).await.unwrap();
        }
        assert_eq!(db.get_mergequeue().await.unwrap().len(), 0);
        assert_eq!(db.get_incomplete().await.unwrap().len(), 0);
        assert_eq!(db.get_complete().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn replace_incomplete_wo_interval() {
        let db = Database::in_memory(1, 0).await.unwrap();

        let rec: Vec<RecordAdd> = (0..10).map(|_| record()).collect();
        db.insert_many(&rec).await.unwrap();
        for r in rec.iter() {
            db.replace_incomplete(r).await.unwrap();
        }
        assert_eq!(db.get_mergequeue().await.unwrap().len(), 10);
        assert_eq!(db.get_incomplete().await.unwrap().len(), 0);
        assert_eq!(db.get_complete().await.unwrap().len(), 0);
        for r in rec.iter() {
            db.replace_incomplete(r).await.unwrap();
        }
        assert_eq!(db.get_mergequeue().await.unwrap().len(), 0);
        assert_eq!(db.get_incomplete().await.unwrap().len(), 10);
        assert_eq!(db.get_complete().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn replace_incomplete_w_interval() {
        let db = Database::in_memory(1, 30).await.unwrap();

        let rec: Vec<RecordAdd> = (0..10).map(|_| record()).collect();
        db.insert_many(&rec).await.unwrap();
        for r in rec.iter() {
            db.replace_incomplete(r).await.unwrap();
        }
        assert_eq!(db.get_mergequeue().await.unwrap().len(), 0);
        assert_eq!(db.get_incomplete().await.unwrap().len(), 0);
        assert_eq!(db.get_complete().await.unwrap().len(), 0);
        for r in rec.iter() {
            db.replace_incomplete(r).await.unwrap();
        }
        assert_eq!(db.get_mergequeue().await.unwrap().len(), 0);
        assert_eq!(db.get_incomplete().await.unwrap().len(), 10);
        assert_eq!(db.get_complete().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn replace_complete() {
        let db = Database::in_memory(1, 30).await.unwrap();

        let rec: Vec<RecordAdd> = (0..10).map(|_| record()).collect();
        db.insert_many(&rec).await.unwrap();
        for r in rec.iter() {
            db.replace_complete(r).await.unwrap();
        }
        assert_eq!(db.get_mergequeue().await.unwrap().len(), 0);
        assert_eq!(db.get_incomplete().await.unwrap().len(), 0);
        assert_eq!(db.get_complete().await.unwrap().len(), 10);
    }

    #[tokio::test]
    async fn lastcheck() {
        let db = Database::in_memory(1, 30).await.unwrap();

        assert!(db.get_lastcheck().await.unwrap().is_none());
        let now = Utc::now();
        db.set_lastcheck(now).await.unwrap();
        assert_eq!(db.get_lastcheck().await.unwrap().unwrap(), now);
    }
}
