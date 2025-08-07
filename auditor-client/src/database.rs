// Copyright 2021-2024 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use auditor::domain::{RecordAdd, RecordUpdate};

use sqlx::{QueryBuilder, Sqlite, SqlitePool, sqlite::SqliteJournalMode};

// See https://docs.rs/sqlx/latest/sqlx/struct.QueryBuilder.html#method.push_bind
const BULK_SIZE: usize = 16384;

fn is_path_valid(path: &Path) -> bool {
    path.to_str().is_some_and(|s| !s.is_empty()) && path.try_exists().is_ok()
}

/// A Wrapper around an SQLite database
///
/// It manages two separate queues: one for inserts (`RecordAdd`) and one for updates
/// (`RecordUpdate`).
#[derive(Clone)]
pub(crate) struct Database {
    db_pool: SqlitePool,
}

impl Database {
    /// Construct new database object
    #[tracing::instrument(name = "Initializing sqlite database connection", level = "debug")]
    pub(crate) async fn new<S: AsRef<str> + fmt::Debug>(path: S) -> Result<Database, sqlx::Error> {
        // Sqlx gives us no error on empty paths...
        // Do some checks
        if !is_path_valid(&PathBuf::from(path.as_ref())) {
            tracing::error!("Invalid path for database: {:?}", path);
            return Err(sqlx::Error::Io(std::io::Error::from(
                std::io::ErrorKind::Other,
            )));
        };
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

    /// Insert a single record into the "insert" queue
    #[tracing::instrument(
        name = "Inserting record into database",
        level = "debug",
        skip(self, record),
        fields(record_id = %record.record_id)
    )]
    pub(crate) async fn insert(&self, record: &RecordAdd) -> Result<(), sqlx::Error> {
        let record = bincode::serialize(record).expect("Should never fail on a record");
        sqlx::query!(
            r#"INSERT OR IGNORE INTO inserts (record) VALUES ($1)"#,
            record
        )
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    /// Insert a vector of records into the "insert" queue
    #[tracing::instrument(
        name = "Bulk inserting records into database",
        level = "debug",
        skip(self, records)
    )]
    pub(crate) async fn insert_many(&self, records: &[RecordAdd]) -> Result<(), sqlx::Error> {
        for chunk in records.chunks(BULK_SIZE) {
            let mut query_builder: QueryBuilder<Sqlite> =
                QueryBuilder::new("INSERT OR IGNORE INTO inserts (record) ");
            let blobs = chunk
                .iter()
                .map(|r| bincode::serialize(&r).expect("Should never fail on a record"));
            query_builder.push_values(blobs, |mut b, blob| {
                b.push_bind(blob);
            });
            query_builder.build().execute(&self.db_pool).await?;
        }
        Ok(())
    }

    /// Insert a single record into the "update" queue
    #[tracing::instrument(
        name = "Updating record in database",
        level = "debug",
        skip(self, record),
        fields(record_id = %record.record_id)
    )]
    pub(crate) async fn update(&self, record: &RecordUpdate) -> Result<(), sqlx::Error> {
        let record = bincode::serialize(record).expect("Should never fail on a record");
        sqlx::query!(r#"INSERT INTO updates (record) VALUES ($1)"#, record)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    /// Delete all records from the "insert" queue with a row id lower or equal to `rowid`
    #[allow(dead_code)]
    #[tracing::instrument(name = "Deleting records from database", level = "debug", skip(self))]
    pub(crate) async fn delete_inserts_le(&self, rowid: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"DELETE FROM inserts WHERE rowid<=$1"#, rowid)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    /// Delete a single record from the "insert" queue
    #[tracing::instrument(name = "Deleting record from database", level = "debug", skip(self))]
    pub(crate) async fn delete_insert(&self, rowid: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"DELETE FROM inserts WHERE rowid=$1"#, rowid)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    /// Delete a single record from the "update" queue
    #[tracing::instrument(name = "Deleting record from database", level = "debug", skip(self))]
    pub(crate) async fn delete_update(&self, rowid: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"DELETE FROM updates WHERE rowid=$1"#, rowid)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    /// Returns all records in the "insert" queue along with their rowids
    #[tracing::instrument(
        name = "Getting insert records from database",
        level = "debug",
        skip(self)
    )]
    pub(crate) async fn get_inserts(&self) -> Result<Vec<(i64, RecordAdd)>, sqlx::Error> {
        struct Row {
            rowid: i64,
            record: Vec<u8>,
        }
        let rows: Vec<Row> = sqlx::query_as!(
            Row,
            r#"SELECT rowid, record FROM inserts ORDER BY rowid ASC"#
        )
        .fetch_all(&self.db_pool)
        .await?;
        let records = rows
            .into_iter()
            .map(|Row { rowid, record }| {
                (rowid, bincode::deserialize::<RecordAdd>(&record).unwrap())
            })
            .collect();
        Ok(records)
    }

    /// The same as [`get_inserts`] but for the "update" queue
    #[tracing::instrument(
        name = "Getting update records from database",
        level = "debug",
        skip(self)
    )]
    pub(crate) async fn get_updates(&self) -> Result<Vec<(i64, RecordUpdate)>, sqlx::Error> {
        struct Row {
            rowid: i64,
            record: Vec<u8>,
        }
        let rows: Vec<Row> = sqlx::query_as!(
            Row,
            r#"SELECT rowid, record FROM updates ORDER BY rowid ASC"#
        )
        .fetch_all(&self.db_pool)
        .await?;
        let records = rows
            .into_iter()
            .map(|Row { rowid, record }| {
                (
                    rowid,
                    bincode::deserialize::<RecordUpdate>(&record).unwrap(),
                )
            })
            .collect();
        Ok(records)
    }

    /// Retrieve records from the "updates" queue, for which the row is less or equal than `rowid`
    #[allow(dead_code)]
    #[tracing::instrument(
        name = "Getting update records from database",
        level = "debug",
        skip(self)
    )]
    pub(crate) async fn get_updates_le(
        &self,
        rowid: i64,
    ) -> Result<Vec<(i64, RecordUpdate)>, sqlx::Error> {
        struct Row {
            rowid: i64,
            record: Vec<u8>,
        }
        let rows: Vec<Row> = sqlx::query_as!(
            Row,
            r#"SELECT rowid, record FROM updates WHERE rowid<=$1 ORDER BY rowid ASC"#,
            rowid
        )
        .fetch_all(&self.db_pool)
        .await?;
        let records = rows
            .into_iter()
            .map(|Row { rowid, record }| {
                (
                    rowid,
                    bincode::deserialize::<RecordUpdate>(&record).unwrap(),
                )
            })
            .collect();
        Ok(records)
    }

    /// Returns the highest rowid in the "update" queue, or `None` if it's empty
    #[tracing::instrument(name = "Getting highest update id", level = "debug", skip(self))]
    pub(crate) async fn get_last_update_rowid(&self) -> Result<Option<i64>, sqlx::Error> {
        struct Row {
            id: Option<i64>,
        }
        let row = sqlx::query_as!(Row, r#"SELECT max(rowid) as id FROM updates"#)
            .fetch_one(&self.db_pool)
            .await?;
        Ok(row.id)
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
        let db = Database::new("sqlite://:memory:").await.unwrap();
        let rec = record();

        db.insert(&rec).await.unwrap();
        let mut res = db.get_inserts().await.unwrap();

        let (_, res) = res.pop().unwrap();
        assert_eq!(Record::from(res), Record::from(rec));
    }

    #[tokio::test]
    async fn update_get() {
        let db = Database::new("sqlite://:memory:").await.unwrap();
        let rec = record();

        db.update(&rec).await.unwrap();
        let mut res = db.get_updates().await.unwrap();

        let (_, res) = res.pop().unwrap();
        assert_eq!(Record::from(res), Record::from(rec));
    }

    #[tokio::test]
    async fn insert_many_get() {
        let db = Database::new("sqlite://:memory:").await.unwrap();
        let recs: Vec<RecordAdd> = (0..10).map(|_| record()).collect();

        db.insert_many(&recs).await.unwrap();
        let res = db.get_inserts().await.unwrap();

        assert_eq!(res.len(), 10);
        assert_eq!(recs.len(), 10);
        res.into_iter()
            .map(|(_, r)| r)
            .zip(recs)
            .for_each(|(a, b)| assert_eq!(Record::from(a), Record::from(b)));
    }

    #[tokio::test]
    async fn update_get_le() {
        let db = Database::new("sqlite://:memory:").await.unwrap();
        let recs: Vec<_> = (0..10).map(|_| record()).collect();

        for r in recs.iter() {
            db.update(r).await.unwrap()
        }
        let res = db.get_updates_le(5).await.unwrap();

        assert_eq!(res.len(), 5);
        assert_eq!(recs.len(), 10);
        res.into_iter()
            .map(|(_, r)| r)
            .zip(recs)
            .for_each(|(a, b)| assert_eq!(Record::from(a), Record::from(b)));
    }

    #[tokio::test]
    async fn insert_many_delete() {
        let db = Database::new("sqlite://:memory:").await.unwrap();
        let recs: Vec<RecordAdd> = (0..10).map(|_| record()).collect();

        db.insert_many(&recs).await.unwrap();
        db.delete_inserts_le(5).await.unwrap();
        let res = db.get_inserts().await.unwrap();

        assert_eq!(res.len(), 5);
        res.into_iter()
            .map(|(_, r)| r)
            .zip(recs.into_iter().skip(5))
            .for_each(|(a, b)| assert_eq!(Record::from(a), Record::from(b)));
    }

    #[tokio::test]
    async fn update_delete() {
        let db = Database::new("sqlite://:memory:").await.unwrap();
        let mut recs: Vec<_> = (0..10).map(|_| record()).collect();

        for r in recs.iter() {
            db.update(r).await.unwrap()
        }
        db.delete_update(5).await.unwrap();
        let res = db.get_updates().await.unwrap();

        recs.remove(4);
        assert_eq!(res.len(), 9);
        res.into_iter()
            .map(|(_, r)| r)
            .zip(recs)
            .for_each(|(a, b)| assert_eq!(Record::from(a), Record::from(b)));
    }

    #[tokio::test]
    async fn update_rowid() {
        let db = Database::new("sqlite://:memory:").await.unwrap();
        let recs: Vec<_> = (0..10).map(|_| record()).collect();

        for r in recs.iter() {
            db.update(r).await.unwrap()
        }
        let rowid = db.get_last_update_rowid().await.unwrap().unwrap();

        assert_eq!(rowid, 10);
    }
}
