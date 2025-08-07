// Copyright 2021-2024 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! This module provides a client to interact with an Auditor instance.
//!
//! # Tutorial
//! This section walks you through several basic usecases of the Auditor software.
//!
//! Auditor is designed around so-called records, which are the unit of accountable resources.
//! Records are created and pushed to Auditor, which stores them in a database.
//! These records can then be requested again from Auditor to take an action
//! based on the information stored in the records.
//!
//! A record consists of a unique identifier and meta information
//! that provides some context (associated site, group, user, ...).
//! Furthermore, a record also contains an arbitrary number of `components`
//! that are to be accounted for (CPU, RAM, Disk, ...) and the amount of each of these components.
//! The components can optionally be enhanced with `scores`, which are floating point values
//! that put components of the same kind, but different performance in relation to each other.
//!
//! ## Creating a Record
//!
//! ```
//! use auditor::domain::{Component, RecordAdd, Score};
//! use chrono::{DateTime, TimeZone, Utc};
//! use std::collections::HashMap;
//!
//! # fn main() -> Result<(), anyhow::Error> {
//! // Define unique identifier
//! let record_id = "record-1"; // Must be unique for all records in Auditor!
//!
//! // Time when the resource became available
//! let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
//!
//! // Create a component (10 CPU cores)
//! // and attache a score (HEPSPEC06) to it
//! let component_cpu = Component::new("CPU", 10)?
//!     .with_score(Score::new("HEPSPEC06", 9.2)?);
//!
//! // Create a second component (32 GB memory)
//! let component_mem = Component::new("MEM", 32)?;
//!
//! // Store components in a vector
//! let components = vec![component_cpu, component_mem];
//!
//! // Create meta information
//! let mut meta = HashMap::new();
//! meta.insert("site_id", vec!["site1"]);
//! meta.insert("features", vec!["ssd", "gpu"]);
//!
//! let record = RecordAdd::new(record_id, meta, components, start_time)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Connecting to Auditor
//!
//! The [`AuditorClientBuilder`] is used to build an [`AuditorClient`] object
//! that can be used for interacting with Auditor.
//!
//! ```
//! use auditor_client::AuditorClientBuilder;
//! # use auditor_client::ClientError;
//!
//! # fn main() -> Result<(), ClientError> {
//! let client = AuditorClientBuilder::new()
//!     .address(&"localhost", 8000)
//!     .timeout(20)
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Connecting to Auditor using tls
//!
//! The [`AuditorClientBuilder`] is used to build an [`AuditorClient`] object
//! that can be used for interacting with Auditor with tls enabled.
//!
//! ```no_run
//! # use auditor_client::AuditorClientBuilder;
//! # use auditor_client::ClientError;
//! #
//! # fn main() -> Result<(), ClientError> {
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .with_tls("client_cert_path", "client_key_path", "ca_cert_path")
//! #     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Pushing one record to Auditor
//!
//! Assuming that a record and a client were already created,
//! the record can be pushed to Auditor with
//!
//! ```no_run
//! # use auditor_client::{AuditorClientBuilder, ClientError};
//! # use auditor::domain::RecordAdd;
//! # use chrono::{DateTime, TimeZone, Utc};
//! # use std::collections::HashMap;
//! # #[tokio::main]
//! # async fn main() -> Result<(), anyhow::Error> {
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//! # let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
//! # let record = RecordAdd::new("record-1", HashMap::new(), vec![], start_time)?;
//! client.add(&record).await?;
//! # Ok(())
//! # }
//! ```
//!
//!  ## Pushing multiple records to AuditorClientBuilder
//!
//!  Assuming that list of records and a client were already created,
//!  the records can be pushed to Auditor with_ymd_and_hms
//!
//! ```no_run
//! # use auditor_client::{AuditorClientBuilder, ClientError};
//! # use auditor::domain::RecordAdd;
//! # use chrono::{DateTime, TimeZone, Utc};
//! # use std::collections::HashMap;
//! # #[tokio::main]
//! # async fn main() -> Result<(), anyhow::Error> {
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//! # let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
//! # let records: Vec<RecordAdd> = (0..5)
//! #    .map(|i| RecordAdd::new(&format!("record-{}", i), HashMap::new(), vec![], start_time))
//! #    .collect::<Result<_, _>>()?;
//! client.bulk_insert(&records).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Updating records in Auditor
//!
//! Auditor accepts incomplete records. In particular, the stop time can be missing.
//! These records can be updated at a later time, by adding the same record which includes a stop time.
//! Note that the `record_id` must match the one already in the database!
//! Fields other than the stop time cannot be updated.
//!
//! ```no_run
//! # use auditor_client::{AuditorClientBuilder, ClientError};
//! use auditor::domain::RecordUpdate;
//! use chrono::{DateTime, TimeZone, Utc};
//! # use std::collections::HashMap;
//! # #[tokio::main]
//! # async fn main() -> Result<(), anyhow::Error> {
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//!
//! let stop_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
//! let record = RecordUpdate::new("record-1", HashMap::new(), vec![], stop_time)?;
//!
//! client.update(&record).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Receiving all records from Auditor
//!
//! The complete set of records can be retrieved from Auditor with the `get()` method:
//!
//! ```no_run
//! # use auditor_client::{AuditorClientBuilder, ClientError};
//! # #[tokio::main]
//! # async fn main() -> Result<(), ClientError> {
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//! let records = client.get().await?;
//! # Ok(())
//! # }
//! ```
//!
//!
//! ## Receiving all records started/stopped since a given timestamp
//!
//! (Deprecated: Use the `advanced_query` function instead)
//!
//! Instead of retrieving all records, the query can be limited to records
//! that have been started or stopped since a given timestamp:
//!
//! ```no_run
//! # use auditor_client::{AuditorClientBuilder, ClientError};
//! use chrono::{DateTime, TimeZone, Utc};
//! # #[tokio::main]
//! # async fn main() -> Result<(), ClientError> {
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//!
//! let since: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
//!
//! let records_started_since = client.get_started_since(&since).await?;
//! let records_stopped_since = client.get_stopped_since(&since).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Advanced Query
//! Records can be queried using fields and operators.
//!
//! ### Template Query
//!
//! The table shows the fields and the corresponding operators available for each field with which a query can be built.
//!
//! ```text
//! GET /records?<field>[<operator>]=<value>
//! ```
//!
//! #### Operators
//! - `gt` (greater than)
//! - `gte` (greater than or equal to)
//! - `lt` (less than)
//! - `lte` (less than or equal to)
//! - `equals` (equal to)
//!
//! #### Meta Operators
//! - `c` (contains)
//! - `dnc` (does not contain)
//!
//! #### SortBy Operators
//! - `asc` (ascending order)
//! - `desc` (descending order)
//!
//! #### SortBy Column names
//! You can specify the column on which the sorting must happen
//! The following columns are supported for sortby option
//! - start_time
//! - stop_time
//! - runtime
//! - record_id
//!
//!| Field        | Description                                                            | Operators                              | Examples (query representation)            |
//!|--------------|------------------------------------------------------------------------|----------------------------------------|--------------------------------------------|
//!| `record_id`  | Retrieve the exact record using `record_id`                            |                                        | `record_id-<record_id>`                    |
//!| `start_time` | Start time of the event (`DateTime<Utc>`)                              | `gt`, `gte`, `lt`, `lte`               | `start_time[gt]=<timestamp>`               |
//!| `stop_time`  | Stop time of the event (`DateTime<Utc>`)                               | `gt`, `gte`, `lt`, `lte`               | `stop_time[gt]=<timestamp>`                |
//!| `runtime`    | Runtime of the event (in seconds)                                      | `gt`, `gte`, `lt`, `lte`               | `runtime[gt]=<u64>`                        |
//!| `meta`       | Meta information (<meta_key>, MetaOperator(<meta_value>))              | `c`, `dnc`                             | `meta[<meta_key>][c]=<meta_value>`         |
//!| `component`  | Component identifier (<component_name>, Operator(<component_amount>))  | `gt`, `gte`, `lt`, `lte`, `equals`     | `component[<component_name>][gt]=<amount>` |
//!| `sort_by`    | Sort query results (SortBy(<column_name>))                             | `asc`, `desc`                          | `sort_by[desc]=<column_name>`              |
//!| `limit`      | limit query records (number)                                           |                                        | `limit=5000`                               |
//!
//! Meta field can be used to query records by specifying the meta key and [`MetaOperator`]  must be used
//! to specify meta values. The [`MetaOperator`] must be used to specify whether the value is
//! contained or is not contained for the specific Metakey.
//!
//! Component field can be used to query records by specifying the component name (CPU) and ['Operator'] must be used
//! to specify the amount.
//!
//! To query records based on a range, specify the field with two operators
//! Either with gt or gte and lt or lte.
//!
//! For example, to query records with start_time ranging between two timestamps:
//!
//! ```text
//! GET records?start_time[gt]=timestamp1&start_time[lt]=timestamp2
//! ```
//!
//! ## QueryBuilder
//!
//! Below are the examples to query records using QueryBuilder methods. It helps to build query string which can be passed
//! as an argument to advanced_query function to get the records.
//!
//! ### Example 1:
//!
//! Constructs an empty [`QueryBuilder`] to query all records
//!
//! ```no_run
//! # use auditor_client::{QueryBuilder, AuditorClientBuilder, ClientError};
//! # #[tokio::main]
//! # async fn main() -> Result<(), ClientError> {
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//! let records = QueryBuilder::new()
//!                 .get(client)
//!                 .await?;
//! # Ok(())
//! # }
//! ```
//!
//! The query string would look like
//!
//! ```text
//! GET records
//! ```
//! ### Example 2:
//!
//! Constructs a QueryBuilder with a start time operator that specifies
//! a range from `datetime_utc_gte` to `datetime_utc_lte`.
//!
//!
//! ```no_run
//! # use auditor_client::{QueryBuilder, Operator, AuditorClientBuilder, ClientError};
//! # use chrono::{Utc, TimeZone};
//! # #[tokio::main]
//! # async fn main() -> Result<(), ClientError> {
//! let datetime_utc_gte = Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap();
//! let datetime_utc_lte = Utc.with_ymd_and_hms(2022, 8, 4, 9, 47, 0).unwrap();
//!
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//! let records = QueryBuilder::new()
//!     .with_start_time(
//!        Operator::default()
//!            .gte(datetime_utc_gte.into())
//!            .lte(datetime_utc_lte.into()),
//!    )
//!    .get(client)
//!    .await?;
//! # Ok(())
//! # }
//! ```
//!
//! The query string would look like:
//!
//! ```text
//! GET records?start_time[lte]=datetime_utc_lte&start_time[gte]=datetime_utc_gte
//! ```
//!
//! ### Example 3:
//!
//! Constructs a QueryBuilder with start time, stop time, and runtime operators,
//! specifying ranges for each.
//!
//! ```no_run
//! # use auditor_client::{QueryBuilder, Operator, AuditorClientBuilder, ClientError};
//! use chrono::{Utc, TimeZone};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), ClientError> {
//! let datetime_utc_gte = Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap();
//! let datetime_utc_lte = Utc.with_ymd_and_hms(2022, 8, 4, 9, 47, 0).unwrap();
//! let runtime_gte: u64 = 100000;
//! let runtime_lte: u64 = 200000;
//!
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//! let records = QueryBuilder::new()
//!     .with_start_time(
//!         Operator::default()
//!             .gte(datetime_utc_gte.into())
//!             .lte(datetime_utc_lte.into()),
//!     )
//!     .with_stop_time(
//!         Operator::default()
//!             .gte(datetime_utc_gte.into())
//!             .lte(datetime_utc_lte.into()),
//!     )
//!     .with_runtime(
//!         Operator::default()
//!             .gte(runtime_gte.into())
//!             .lte(runtime_lte.into()),
//!     )
//!     .get(client)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! The query string would look like
//!
//! ```text
//! GET
//! records?start_time[gte]=datetime_utc_gte&start_time[lte]=datetime_utc_lte&stop_time[gte]=datetime_utc_gte&stop_time[lte]=datetime_utc_lte&runtime[gte]=runtime_gte&runtime[lte]=runtime_lte
//! ```
//!
//! ### Example 4:
//!
//! Constructs a QueryBuilder with a meta query specifying "site_id" should contain "site1" value
//! and a start time operator specifying a maximum datetime.
//!
//! ```no_run
//! # use auditor_client::{QueryBuilder, Operator, MetaQuery, MetaOperator, AuditorClientBuilder,
//! ClientError};
//! use chrono::{Utc, TimeZone};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), ClientError> {
//! let datetime_utc_lte = Utc.with_ymd_and_hms(2022, 8, 4, 9, 47, 0).unwrap();
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//! let records = QueryBuilder::new()
//!     .with_meta_query(
//!         MetaQuery::new().meta_operator(
//!             "site_id".to_string(),
//!             MetaOperator::default().contains(vec!["site1".to_string()]),
//!         )
//!     )
//!     .with_start_time(
//!         Operator::default().lte(datetime_utc_lte.into())
//!     )
//!     .get(client)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! The query string would look like:
//!
//! ```text
//! GET records?meta[site_id][c][0]=site1&start_time[lte]=datetime_utc_lte
//! ```
//!
//! ### Example 5:
//!
//! Constructs a QueryBuilder with a component query specifying an equality condition for the "CPU" field.
//!
//! ```no_run
//! use auditor_client::{QueryBuilder, Operator, ComponentQuery, AuditorClientBuilder, ClientError};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), ClientError> {
//! let count: u8 = 10; // querying records whose cpu amount is 10
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//! let records = QueryBuilder::new()
//!     .with_component_query(
//!         ComponentQuery::new().component_operator(
//!             "CPU".to_string(),
//!             Operator::default().equals(count.into()),
//!         )
//!     )
//!     .get(client)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! The query string would look like
//!
//! ```text
//! GET records?component[CPU][equals]=count
//! ```
//!
//!//! ### Example 6:
//!
//! Constructs a QueryBuilder which sorts the record in descending order by stop_time and limits the query results by 500 records
//!
//! ```no_run
//! use auditor_client::{QueryBuilder, AuditorClientBuilder, ClientError, SortBy};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), ClientError> {
//! let number: u64 = 500;
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//! let records = QueryBuilder::new()
//!     .sort_by(SortBy::new().descending("stop_time".to_string()))
//!     .limit(number)
//!     .get(client)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! The query string would look like
//!
//! ```text
//! GET records?sort_by[desc]=stop_time&limit=number
//! ```
//!
//! ### Example 7:
//!
//! Constructs a QueryBuilder to retrieve one record using record id
//!
//! ```no_run
//! use auditor_client::{QueryBuilder, AuditorClientBuilder, ClientError};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), ClientError> {
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//! let record_id = "record-1".to_string();
//! let records = client.get_single_record(record_id).await?;
//! # Ok(())
//! # }
//! ```
//!
//! The query string would look like
//!
//! ```text
//! GET record/record-1
//! ```
//!
//! ## Warning
//! `equals` operator is only available for querying components. It cannot be used for time based
//! queries
//!
//! If the query is directly appended to the URL, please make sure that the datetime value is urlencoded
//!
//! ## Checking the health of Auditor
//!
//! The health of Auditor can be checked with
//!
//! ```no_run
//! # use auditor_client::{AuditorClientBuilder, ClientError};
//! # #[tokio::main]
//! # async fn main() -> Result<(), ClientError> {
//! # let client = AuditorClientBuilder::new()
//! #     .address(&"localhost", 8000)
//! #     .timeout(20)
//! #     .build()?;
//! #
//! if client.health_check().await {
//!     println!(":)");
//! } else {
//!     println!(":(");
//! }
//! # Ok(())
//! # }
//! ```

mod constants;
use auditor::{
    constants::ERR_RECORD_EXISTS,
    domain::{Record, RecordAdd, RecordUpdate},
};
use constants::ERR_INVALID_TIME_INTERVAL;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::HashMap;
use tokio::sync::oneshot;
use urlencoding::encode;

mod database;
use database::Database;

use reqwest::{Certificate, Identity};
use std::fs;

use futures::TryStreamExt;
use reqwest_streams::*;
use serde_json::Deserializer;
use std::io::BufReader;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ClientError {
    RecordExists,
    InvalidTimeInterval,
    ReqwestError(reqwest::Error),
    DatabaseError(sqlx::Error),
    Other(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ClientError::RecordExists => ERR_RECORD_EXISTS.to_string(),
                ClientError::InvalidTimeInterval => ERR_INVALID_TIME_INTERVAL.to_string(),
                ClientError::ReqwestError(e) => format!("Reqwest Error: {e}"),
                ClientError::DatabaseError(e) => format!("Database Error: {e}"),
                ClientError::Other(s) => format!("Other client error: {s}"),
            }
        )
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(error: reqwest::Error) -> Self {
        ClientError::ReqwestError(error)
    }
}

impl From<chrono::OutOfRangeError> for ClientError {
    fn from(_: chrono::OutOfRangeError) -> Self {
        ClientError::InvalidTimeInterval
    }
}

impl From<sqlx::Error> for ClientError {
    fn from(error: sqlx::Error) -> Self {
        ClientError::DatabaseError(error)
    }
}

impl From<anyhow::Error> for ClientError {
    fn from(error: anyhow::Error) -> Self {
        ClientError::Other(error.to_string())
    }
}

/// The `AuditorClientBuilder` is used to build an instance of
/// [`AuditorClient`], [`AuditorClientBlocking`] or [`QueuedAuditorClient`].
///
/// # Examples
///
/// Using the `address` and `port` of the Auditor instance:
///
/// ```
/// # use auditor_client::{AuditorClientBuilder, ClientError};
/// #
/// # fn main() -> Result<(), ClientError> {
/// let client = AuditorClientBuilder::new()
///     .address(&"localhost", 8000)
///     .timeout(20)
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// Using an connection string:
///
/// ```
/// # use auditor_client::{AuditorClientBuilder, ClientError};
/// #
/// # fn main() -> Result<(), ClientError> {
/// let client = AuditorClientBuilder::new()
///     .connection_string(&"http://localhost:8000")
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct AuditorClientBuilder {
    address: String,
    database_path: PathBuf,
    timeout: Duration,
    send_interval: Duration,
    tls_config: Option<TlsConfig>,
}

impl AuditorClientBuilder {
    /// Constructor.
    pub fn new() -> AuditorClientBuilder {
        AuditorClientBuilder {
            address: "127.0.0.1:8080".into(),
            database_path: PathBuf::from("sqlite::memory:"),
            timeout: Duration::try_seconds(30).expect("This should never fail"),
            send_interval: Duration::try_seconds(60).expect("This should never fail"),
            tls_config: None,
        }
    }

    /// Set the address and port of the Auditor server.
    ///
    /// # Arguments
    ///
    /// * `address` - Host name / IP address of the Auditor instance.
    /// * `port` - Port of the Auditor instance.
    #[must_use]
    pub fn address<T: AsRef<str>>(mut self, address: &T, port: u16) -> Self {
        self.address = format!("{}:{}", address.as_ref(), port);
        self
    }

    /// Set a connection string of the form ``http://<auditor_address>:<auditor_port>``.
    ///
    /// # Arguments
    ///
    /// * `connection_string` - Connection string.
    #[must_use]
    pub fn connection_string<T: AsRef<str>>(mut self, connection_string: &T) -> Self {
        self.address = connection_string.as_ref().into();
        self
    }

    /// Set a timeout in seconds for HTTP requests.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in seconds.
    #[must_use]
    pub fn timeout(mut self, timeout: i64) -> Self {
        self.timeout = Duration::try_seconds(timeout)
            .unwrap_or_else(|| panic!("Could not convert {timeout} to duration"));
        self
    }

    /// Set an interval in seconds for periodic updates to AUDITOR.
    /// This setting is only relevant to the `QueuedAuditorClient`.
    ///
    /// # Arguments
    ///
    /// * `interval` - Interval in seconds.
    pub fn send_interval(mut self, interval: i64) -> Self {
        self.send_interval = Duration::try_seconds(interval)
            .unwrap_or_else(|| panic!("Could not convert {interval} to duration"));
        self
    }

    /// Set the file path for the persistent storage sqlite db.
    /// This setting is only relevant to the `QueuedAuditorClient`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the database (SQLite) file
    pub fn database_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.database_path = path.as_ref().to_path_buf();
        self
    }

    pub fn with_tls<P: AsRef<Path>>(
        mut self,
        client_cert_path: P,
        client_key_path: P,
        ca_cert_path: P,
    ) -> Self {
        let mut tls_config = TlsConfig::new();

        match (fs::read(client_cert_path), fs::read(client_key_path)) {
            (Ok(client_cert), Ok(client_key)) => {
                match Identity::from_pem(&[client_cert, client_key].concat()) {
                    Ok(identity) => tls_config.identity = Some(identity),
                    Err(e) => {
                        eprintln!("Failed to create identity from client cert and key: {e}")
                    }
                }
            }
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("Failed to read client certificate or key: {e}");
            }
        }

        match fs::read(ca_cert_path) {
            Ok(ca_cert) => match Certificate::from_pem(&ca_cert) {
                Ok(ca_certificate) => tls_config.ca_certificate = Some(ca_certificate),
                Err(e) => eprintln!("Failed to parse CA certificate PEM: {e}"),
            },
            Err(e) => eprintln!("Failed to read CA certificate file: {e}"),
        }

        self.tls_config = Some(tls_config);
        self
    }

    /// Build an [`AuditorClient`] from `AuditorClientBuilder`.
    ///
    /// # Errors
    ///
    /// * [`ClientError::InvalidTimeInterval`] - If the timeout duration is less than zero.
    /// * [`ClientError::ReqwestError`] - If there was an error building the HTTP client.
    pub fn build(self) -> Result<AuditorClient, ClientError> {
        let client = match self.tls_config.clone() {
            Some(tls_config) => reqwest::ClientBuilder::new()
                .identity(tls_config.identity.expect(
                    "Error while setting up the client identity using client cert and key pem",
                ))
                .add_root_certificate(
                    tls_config
                        .ca_certificate
                        .expect("Error while setting up the root certificate"),
                )
                .timeout(self.timeout.to_std()?)
                .build()?,
            None => reqwest::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .timeout(self.timeout.to_std()?)
                .build()?,
        };

        // The only reason we check if the self.address contains the protocol is because
        // of the unit tests that uses mock to create connection uri.
        let address = if self.address.starts_with("http://") || self.address.starts_with("https://")
        {
            self.address.clone()
        } else {
            let scheme = if self.tls_config.is_some() {
                "https"
            } else {
                "http"
            };
            format!("{}://{}", scheme, self.address)
        };

        Ok(AuditorClient { address, client })
    }

    /// Build a [`QueuedAuditorClient`] from `AuditorClientBuilder`.
    ///
    /// # Errors
    ///
    /// * [`ClientError::InvalidTimeInterval`] - If the timeout duration or send interval is less than zero.
    /// * [`ClientError::ReqwestError`] - If there was an error building the HTTP client.
    /// * [`ClientError::DatabaseError`] - If there was an error while opening or creating the
    ///   database
    pub async fn build_queued(self) -> Result<QueuedAuditorClient, ClientError> {
        let interval = self.send_interval;
        let client = QueuedAuditorClient::new(
            Database::new(
                self.database_path
                    .to_str()
                    .ok_or(ClientError::Other(format!(
                        "Path {:?} is no valid UTF-8",
                        self.database_path
                    )))?,
            )
            .await?,
            self.build()?,
            interval.to_std()?,
        );
        Ok(client)
    }

    /// Build an [`AuditorClientBlocking`] from `AuditorClientBuilder`.
    ///
    /// # Errors
    ///
    /// * [`ClientError::InvalidTimeInterval`] - If the timeout duration is less than zero.
    /// * [`ClientError::ReqwestError`] - If there was an error building the HTTP client.
    ///
    /// # Panics
    ///
    /// This method panics if it is called from an async runtime.
    pub fn build_blocking(self) -> Result<AuditorClientBlocking, ClientError> {
        let client = match self.tls_config.clone() {
            Some(tls_config) => reqwest::blocking::ClientBuilder::new()
                .identity(tls_config.identity.expect(
                    "Error while setting up the client identity using client cert and key pem",
                ))
                .add_root_certificate(
                    tls_config
                        .ca_certificate
                        .expect("Error while setting up the root certificate"),
                )
                .timeout(self.timeout.to_std()?)
                .build()?,
            None => reqwest::blocking::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .timeout(self.timeout.to_std()?)
                .build()?,
        };

        // The only reason we check if the self.address contains the protocol is because
        // of the unit tests that uses mock to create connection uri.
        let address = if self.address.starts_with("http://") || self.address.starts_with("https://")
        {
            self.address.clone()
        } else {
            let scheme = if self.tls_config.is_some() {
                "https"
            } else {
                "http"
            };
            format!("{}://{}", scheme, self.address)
        };

        Ok(AuditorClientBlocking { address, client })
    }
}

#[derive(Debug, Clone)]
struct TlsConfig {
    identity: Option<Identity>,
    ca_certificate: Option<Certificate>,
}

impl TlsConfig {
    fn new() -> Self {
        TlsConfig {
            identity: None,
            ca_certificate: None,
        }
    }
}

impl Default for AuditorClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// `DateTimeUtcWrapper` helps to implement custom serialization to serialize `DateTime<Utc>`
/// to rfc3339, so that it can be used to correctly encode the query string.
#[derive(serde::Deserialize, Debug, Default, Clone)]
pub struct DateTimeUtcWrapper(pub DateTime<Utc>);

/// Implementation of the `Serialize` trait for DateTimeUtcWrapper.
impl Serialize for DateTimeUtcWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_rfc3339())
    }
}

/// The `QueryParameters` is used to build query parameters which allows to query records from
/// the database using advanced_query function.
#[derive(serde::Deserialize, serde::Serialize, Debug, Default, Clone)]
pub struct QueryParameters {
    /// Specifies the record id to query the exact record from the database
    pub record_id: Option<String>,
    /// Specifies the start time for querying records. It uses the `Operator` enum to
    /// define time-based operations.
    pub start_time: Option<Operator>,
    /// Specifies the stop time for querying records. It uses the `Operator` enum to
    /// define time-based operations.
    pub stop_time: Option<Operator>,
    /// Specifies the runtime for querying records. It uses the `Operator` enum to
    /// define time-based operations.
    pub runtime: Option<Operator>,
    /// Specifies the meta values for querying records. It uses the `MetaOperator` enum to
    /// define meta operations.
    pub meta: Option<MetaQuery>,
    /// Specifies the start time for querying records. It uses the `Operator` enum to
    /// define component-based operations.
    pub component: Option<ComponentQuery>,
    // Specifies either to sort the query by ascending or descending order
    pub sort_by: Option<SortBy>,
    /// Specifies the number of query records to be returned
    pub limit: Option<u64>,
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Enum representing different types of values that can be used in query parameters
/// Enum is used instead of generics to specify the type because pyo3 bindings does not contain the equivalent
/// generics implementation.
#[derive(serde::Deserialize, Debug, Clone)]
pub enum Value {
    /// Represents a datetime value
    Datetime(DateTimeUtcWrapper),
    /// Represents a runtime value
    Runtime(u64),
    /// Represents a count value
    Count(u8),
}

/// Implementation of the `Serialize` trait for the `Value` enum.
impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Datetime(datetime) => datetime.serialize(serializer),
            Value::Runtime(runtime) => runtime.serialize(serializer),
            Value::Count(count) => count.serialize(serializer),
        }
    }
}

/// The `Operator` struct is used to specify the operators on the query parameters.
#[derive(serde::Deserialize, serde::Serialize, Debug, Default, Clone)]
pub struct Operator {
    /// Greater than operator.
    pub gt: Option<Value>,
    /// Lesser than operator.
    pub lt: Option<Value>,
    /// Greater than or equals operator.
    pub gte: Option<Value>,
    /// Lesser than or equals operator.
    pub lte: Option<Value>,
    /// Equals operator.
    pub equals: Option<Value>,
}

/// Implementation of methods for the `Operator` struct to set various operators.
impl Operator {
    pub fn gt(mut self, value: Value) -> Self {
        self.gt = Some(value);
        self
    }

    pub fn lt(mut self, value: Value) -> Self {
        self.lt = Some(value);
        self
    }

    pub fn gte(mut self, value: Value) -> Self {
        self.gte = Some(value);
        self
    }

    pub fn lte(mut self, value: Value) -> Self {
        self.lte = Some(value);
        self
    }

    pub fn equals(mut self, value: Value) -> Self {
        if !matches!(value, Value::Datetime(_)) {
            self.equals = Some(value);
            self
        } else {
            self
        }
    }
}

// Implementations of conversion traits for the `Value` enum.

/// Conversion from chrono DateTime to Value::Datetime.
impl From<chrono::DateTime<Utc>> for Value {
    fn from(item: chrono::DateTime<Utc>) -> Self {
        Value::Datetime(DateTimeUtcWrapper(item))
    }
}

/// Conversion from u64 to Value::Runtime.
impl From<u64> for Value {
    fn from(item: u64) -> Self {
        Value::Runtime(item)
    }
}

/// Conversion from u8 to Value::Count.
impl From<u8> for Value {
    fn from(item: u8) -> Self {
        Value::Count(item)
    }
}

/// The `QueryBuilder` is used to construct `QueryParameters` using the builder pattern.
/// It is used to fetch records using query parameters such as start_time, stop_time etc.
///
/// # Examples
///
/// ```
/// use auditor_client::{QueryBuilder, Operator, MetaQuery, ComponentQuery, SortBy};
///
/// // Create a new QueryBuilder instance.
/// let query_builder = QueryBuilder::new()
///     .with_start_time(Operator::default()) // Set start time operator.
///     .with_stop_time(Operator::default())  // Set stop time operator.
///     .with_runtime(Operator::default())    // Set runtime operator.
///     .with_meta_query(MetaQuery::new())    // Set meta query.
///     .with_component_query(ComponentQuery::new())  // Set component query.
///     .sort_by(SortBy::new()) // Set sort by options
///     .limit(1000); // Limit the number of queries
///
/// // For querying all records, just create an empty QueryBuilder instance without operators
/// let query_builder = QueryBuilder::new();
///
/// // Build the query string.
/// let query_string = query_builder.build();
/// println!("Generated query string: {}", query_string);
/// ```
///
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    /// Query parameters to be built.
    pub query_params: QueryParameters,
}

impl QueryBuilder {
    /// Creates a new instance of the QueryBuilder with default parameters.
    pub fn new() -> Self {
        QueryBuilder {
            query_params: QueryParameters {
                record_id: None,
                start_time: None,
                stop_time: None,
                runtime: None,
                meta: None,
                component: None,
                sort_by: None,
                limit: None,
            },
        }
    }

    /// Sets the exact record to be queried from the database using record id
    pub fn with_record_id(mut self, record_id: String) -> Self {
        self.query_params.record_id = Some(record_id);
        self
    }

    /// Sets the start time in the query parameters.
    pub fn with_start_time(mut self, time_operator: Operator) -> Self {
        self.query_params.start_time = Some(time_operator);
        self
    }

    /// Sets the stop time in the query parameters.
    pub fn with_stop_time(mut self, time_operator: Operator) -> Self {
        self.query_params.stop_time = Some(time_operator);
        self
    }

    /// Sets the runtime in the query parameters.
    pub fn with_runtime(mut self, time_operator: Operator) -> Self {
        self.query_params.runtime = Some(time_operator);
        self
    }

    /// Sets the meta query in the query parameters.
    pub fn with_meta_query(mut self, meta: MetaQuery) -> Self {
        self.query_params.meta = Some(meta);
        self
    }

    /// Sets the component query in the query parameters.
    pub fn with_component_query(mut self, component: ComponentQuery) -> Self {
        self.query_params.component = Some(component);
        self
    }

    /// Sets the sort_by option for the resulting query
    pub fn sort_by(mut self, sort: SortBy) -> Self {
        self.query_params.sort_by = Some(sort);
        self
    }

    pub fn limit(mut self, number: u64) -> Self {
        self.query_params.limit = Some(number);
        self
    }

    // Executes an asynchronous query using the built parameters.
    ///
    /// # Arguments
    ///
    /// * `client` - An instance of the `AuditorClient` used to perform the query.
    ///
    /// # Returns
    ///
    /// A `Result` containing the vector of records if successful, or a `ClientError` if an error occurs.
    ///
    pub async fn get(&self, client: AuditorClient) -> Result<Vec<Record>, ClientError> {
        let query_string = self.build();
        client.advanced_query(query_string).await
    }

    /// Builds and returns the serialized query string
    pub fn build(&self) -> String {
        serde_qs::to_string(&self.query_params).expect("Failed to serialize query parameters")
    }
}

/// The `MetaQuery` struct represents a set of metadata queries associated with specific query IDs
/// It is used to filter records based on metadata conditions.
#[derive(serde::Deserialize, Debug, Default, Clone)]
pub struct MetaQuery {
    /// HashMap containing query IDs and corresponding metadata operators.
    pub meta_query: HashMap<String, Option<MetaOperator>>,
}

impl MetaQuery {
    /// Creates a new instance of `MetaQuery` with an empty HashMap.
    pub fn new() -> Self {
        MetaQuery {
            meta_query: HashMap::new(),
        }
    }

    /// Adds a new metadata operator to the `MetaQuery` instance for a specific query ID.
    ///
    /// # Arguments
    ///
    /// * `query_id` - A unique identifier for the metadata query.
    /// * `operator` - The metadata operator containing conditions for the query.
    ///
    /// # Returns
    ///
    /// A new `MetaQuery` instance with the added metadata operator.
    pub fn meta_operator(mut self, query_id: String, operator: MetaOperator) -> Self {
        self.meta_query.insert(query_id.to_string(), Some(operator));
        self
    }
}

/// Implementation of the `Serialize` trait for the `MetaQuery` struct.
/// It allows the serialization of `MetaQuery` instances for building query string.
impl Serialize for MetaQuery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.meta_query.serialize(serializer)
    }
}

/// The `MetaOperator` struct represents operators for metadata queries, specifying conditions for filtering.
#[derive(serde::Deserialize, serde::Serialize, Debug, Default, Clone)]
pub struct MetaOperator {
    /// `contains` - Specifies if the meta key contains the value.
    pub c: Option<Vec<String>>,
    /// `does not contain` - Specifies if the meta key does not contain the value.
    pub dnc: Option<Vec<String>>,
}

impl MetaOperator {
    /// Specifies that the metadata query should contain a specific value.
    ///
    /// # Arguments
    ///
    /// * `c` - The value to be contained in the metadata query.
    ///
    /// # Returns
    ///
    /// A new `MetaOperator` instance with the specified condition.
    pub fn contains(mut self, c: Vec<String>) -> Self {
        self.c = Some(c);
        self
    }

    /// Specifies that the metadata query should not contain a specific value.
    ///
    /// # Arguments
    ///
    /// * `dnc` - The value that the metadata query should not contain.
    ///
    /// # Returns
    ///
    /// A new `MetaOperator` instance with the specified condition.
    pub fn does_not_contain(mut self, dnc: Vec<String>) -> Self {
        self.dnc = Some(dnc);
        self
    }
}

/// The `ComponentQuery` struct represents a set of component queries associated with specific query IDs.
/// It is used to filter records based on component-related conditions.
#[derive(serde::Deserialize, Debug, Default, Clone)]
pub struct ComponentQuery {
    /// HashMap containing query IDs and corresponding component operators.
    pub component_query: HashMap<String, Option<Operator>>,
}

impl ComponentQuery {
    /// Creates a new instance of `ComponentQuery` with an empty HashMap.
    pub fn new() -> Self {
        ComponentQuery {
            component_query: HashMap::new(),
        }
    }

    /// Adds a new component operator to the `ComponentQuery` instance for a specific query ID.
    ///
    /// # Arguments
    ///
    /// * `query_id` - A unique identifier for the component query.
    /// * `operator` - The component operator containing conditions for the query.
    ///
    /// # Returns
    ///
    /// A new `ComponentQuery` instance with the added component operator.
    pub fn component_operator(mut self, query_id: String, operator: Operator) -> Self {
        self.component_query
            .insert(query_id.to_string(), Some(operator));
        self
    }
}

/// Implementation of the `Serialize` trait for the `ComponentQuery` struct.
/// It allows the serialization of `ComponentQuery` instances for building query string.
impl Serialize for ComponentQuery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.component_query.serialize(serializer)
    }
}

/// SortBy provides options on sorting the query records
#[derive(serde::Deserialize, serde::Serialize, Debug, Default, Clone)]
pub struct SortBy {
    pub asc: Option<String>,
    pub desc: Option<String>,
}

impl SortBy {
    /// Creates a new instance of `SortBy`
    pub fn new() -> Self {
        Self {
            asc: None,
            desc: None,
        }
    }

    /// Specify the column by which the query records must be sorted in ascending order
    ///
    /// # Arguments
    ///
    /// * `column` - One of four values (`start_time`, `stop_time`, `runtime`, `record_id`)
    ///
    /// # Returns
    ///
    /// A new `SortBy` instance with column name.
    pub fn ascending(mut self, column: String) -> Self {
        self.asc = Some(column);
        self
    }

    /// Specify the column by which the query records must be sorted in descending order
    ///
    /// # Arguments
    ///
    /// * `column` - One of three values (`start_time`, `stop_time`, `runtime`, `record_id`)
    ///
    /// # Returns
    ///
    /// A new `SortBy` instance with column name.
    pub fn descending(mut self, column: String) -> Self {
        self.desc = Some(column);
        self
    }
}

/// The `AuditorClient` handles the interaction with the Auditor instances and allows one to add
/// records to the database, update records in the database and retrieve the records from the
/// database.
///
/// It is constructed using the [`AuditorClientBuilder`].
#[derive(Clone)]
pub struct AuditorClient {
    address: String,
    client: reqwest::Client,
}

impl AuditorClient {
    /// Returns ``true`` if the Auditor instance is healthy, ``false`` otherwise.
    #[tracing::instrument(name = "Checking health of AUDITOR server.", skip(self))]
    pub async fn health_check(&self) -> bool {
        match self
            .client
            .get(format!("{}/health_check", &self.address))
            .send()
            .await
        {
            Ok(s) => s.error_for_status().is_ok(),
            Err(_) => false,
        }
    }

    /// Push a record to the Auditor instance.
    ///
    /// # Errors
    ///
    /// * [`ClientError::RecordExists`] - If the record already exists in the database.
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Sending a record to AUDITOR server.",
        skip(self, record),
        fields(record_id = %record.record_id),
        level = "debug"
    )]
    pub async fn add(&self, record: &RecordAdd) -> Result<(), ClientError> {
        let response = self
            .client
            .post(format!("{}/record", &self.address))
            .header("Content-Type", "application/json")
            .json(record)
            .send()
            .await?;

        if response.text().await? == ERR_RECORD_EXISTS {
            Err(ClientError::RecordExists)
        } else {
            Ok(())
        }
    }

    /// Push multiple record to the Auditor instance as a vec.
    ///
    /// # Errors
    ///
    /// * [`ClientError::RecordExists`] - If the record already exists in the database.
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Sending multiple records to AUDITOR server.",
        skip(self, records)
    )]
    pub async fn bulk_insert(&self, records: &Vec<RecordAdd>) -> Result<(), ClientError> {
        let response = self
            .client
            .post(format!("{}/records", &self.address))
            .header("Content-Type", "application/json")
            .json(records)
            .send()
            .await?;

        if response.text().await? == ERR_RECORD_EXISTS {
            Err(ClientError::RecordExists)
        } else {
            Ok(())
        }
    }

    /// Update an existing record in the Auditor instance.
    ///
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Sending a record update to AUDITOR server.",
        skip(self, record),
        fields(record_id = %record.record_id)
    )]
    pub async fn update(&self, record: &RecordUpdate) -> Result<(), ClientError> {
        self.client
            .put(format!("{}/record", &self.address))
            .header("Content-Type", "application/json")
            .json(record)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// Gets all records from the Auditors database.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(name = "Getting all records from AUDITOR server.", skip(self))]
    pub async fn get(&self) -> Result<Vec<Record>, ClientError> {
        let response = self
            .client
            .get(format!("{}/records", &self.address))
            .send()
            .await?
            .error_for_status()?
            .json_array_stream::<Record>(10000);
        let records: Vec<Record> = response
            .try_collect()
            .await
            .expect("Unexpected error while processing the stream");
        Ok(records)
    }

    /// Get all records in the database with a started timestamp after ``since``.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Getting all records started since a given date from AUDITOR server.",
        skip(self),
        fields(started_since = %since)
    )]
    #[deprecated(since = "0.4.0", note = "please use `advanced_query` instead")]
    pub async fn get_started_since(
        &self,
        since: &DateTime<Utc>,
    ) -> Result<Vec<Record>, ClientError> {
        dbg!(since.to_rfc3339());
        let since_str = since.to_rfc3339();
        let encoded_since = encode(&since_str);
        let response = self
            .client
            .get(format!(
                "{}/records?start_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()
            .await?
            .error_for_status()?
            .json_array_stream::<Record>(10000);
        let records: Vec<Record> = response
            .try_collect()
            .await
            .expect("Unexpected error while processing the stream");
        Ok(records)
    }

    /// Get all records in the database with a stopped timestamp after ``since``.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Getting all records stopped since a given date from AUDITOR server.",
        skip(self),
        fields(started_since = %since)
    )]
    #[deprecated(since = "0.4.0", note = "please use `advanced_query` instead")]
    pub async fn get_stopped_since(
        &self,
        since: &DateTime<Utc>,
    ) -> Result<Vec<Record>, ClientError> {
        let since_str = since.to_rfc3339();
        let encoded_since = encode(&since_str);
        let response = self
            .client
            .get(format!(
                "{}/records?stop_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()
            .await?
            .error_for_status()?
            .json_array_stream::<Record>(10000);
        let records: Vec<Record> = response
            .try_collect()
            .await
            .expect("Unexpected error while processing the stream");
        Ok(records)
    }

    /// Get records from AUDITOR server using custom query.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Getting records from AUDITOR server using custom query",
        skip(self)
    )]
    pub async fn advanced_query(&self, query_string: String) -> Result<Vec<Record>, ClientError> {
        let response = self
            .client
            .get(format!("{}/records?{}", &self.address, query_string))
            .send()
            .await?
            .error_for_status()?
            .json_array_stream::<Record>(10000);
        let records: Vec<Record> = response
            .try_collect()
            .await
            .expect("Unexpected error while processing the stream");
        Ok(records)
    }

    /// Get single record from AUDITOR server using record_id.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Getting a single record from AUDITOR server using record_id",
        skip(self)
    )]
    pub async fn get_single_record(&self, record_id: String) -> Result<Record, ClientError> {
        Ok(self
            .client
            .get(format!("{}/record/{}", &self.address, record_id))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

/// The `QueuedAuditorClient` handles the interaction with the Auditor instances. All
/// data to be sent is transparently saved in a persistent local database.
///
/// It is constructed using [`AuditorClientBuilder::build_queued`] and provides the same
/// interface as [`AuditorClient`].
///
/// When records are sent to Auditor, this client will transparently buffer them in a
/// (persistent) local database.
/// A background task will then periodically send records from the local database to
/// Auditor, deleting them from the local database only after they have been successfully
/// send to Auditor.
///
/// # Notes
/// There are some quirks that need to be observed when using this client:
/// - Since sending and updating records is delayed, there is no guarantee that a record
///   can be retrieved from Auditor right after it has been "sent" by this client.
/// - The background task of this client should be stopped by invoking [`QueuedAuditorClient::stop`]
///   before the client is dropped.
/// - Since methods for sending records like `QueuedAuditorClient::add` only push the records to
///   the local queue, they can only ever raise database errors.
///   Errors like `ClientError::ReqwestError` or `ClientError::RecordExists` can only be triggered
///   by the background send task and will be logged.
///
/// # Examples
/// ```
/// # use auditor_client::{AuditorClientBuilder, ClientError};
/// # use auditor::domain::{RecordAdd, RecordTest};
/// #
/// # async fn foo() -> Result<(), ClientError> {
/// # let record = RecordAdd::try_from(RecordTest::default()).unwrap();
/// let mut client = AuditorClientBuilder::new()
///     .address(&"localhost", 8000)
///     .database_path("sqlite://:memory:")
///     .send_interval(60)
///     .build_queued()
///     .await?;
/// client.add(&record).await?;
/// client.stop().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct QueuedAuditorClient {
    database: Database,
    client: AuditorClient,
    shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    task_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl QueuedAuditorClient {
    /// Constructs the `QueuedAuditorClient` and starts the background send task
    fn new(database: Database, client: AuditorClient, interval: std::time::Duration) -> Self {
        let mut interval = tokio::time::interval(interval);
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let _database = database.clone();
        let _client = client.clone();
        // Note: Since the first tick on interval::tick is immediate,
        // a send is triggered immediately.
        let task_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = interval.tick() => {},
                    result = &mut shutdown_rx => {
                        if let Err(e) = result { tracing::error!("Error: {:?}", e) }
                        break;
                    },
                }
                if let Err(e) = Self::process_queue(&_database, &_client).await {
                    tracing::error!("Processing queue failed with error: {e}");
                }
            }
        });
        Self {
            database,
            client,
            shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
            task_handle: Arc::new(Mutex::new(Some(task_handle))),
        }
    }

    #[tracing::instrument(name = "Process client send queue", skip(database, client))]
    async fn process_queue(database: &Database, client: &AuditorClient) -> Result<(), ClientError> {
        // Most recent update id
        let update_rowid = database.get_last_update_rowid().await?;

        // Send all inserts
        for (rowid, r) in database.get_inserts().await? {
            match client.add(&r).await {
                Ok(_) => {
                    tracing::info!("Successfully sent {} records", r.record_id);
                    database.delete_insert(rowid).await?;
                }
                Err(ClientError::RecordExists) => {
                    tracing::warn!(
                        "Failed sending record to Auditor instance. Record already exists: {}",
                        r.record_id,
                    );
                    database.delete_insert(rowid).await?;
                }
                Err(e) => return Err(e),
            };
        }

        // Send updates
        if let Some(maxid) = update_rowid {
            let updates = database.get_updates().await?;
            for (rowid, u) in updates {
                if rowid > maxid {
                    continue;
                };
                match client.update(&u).await {
                    Ok(_) => {
                        tracing::info!("Successfully updated record {}", u.record_id);
                        database.delete_update(rowid).await?;
                    }
                    Err(e) => return Err(e),
                }
            }
        };
        Ok(())
    }

    /// Stops the background sync task
    #[tracing::instrument(name = "Stop QueuedAuditorClient task", skip(self))]
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        // We cannot hold a MutexGuard across an await and Tokio cannot reason about
        // Drops, so use scopes and Options
        let handle;
        {
            let mut handle_opt = self.task_handle.lock().unwrap();
            if handle_opt.is_none() {
                anyhow::bail!("Send task is already shut down");
            }
            let shutdown_tx = self.shutdown_tx.lock().unwrap().take().unwrap();
            if shutdown_tx.send(()).is_err() {
                anyhow::bail!("Error while sending shutdown.");
            }
            handle = Some(handle_opt.take().unwrap());
        }
        if let Err(e) = handle.unwrap().await {
            anyhow::bail!("Error while waiting on sender task to finish: {:?}", e);
        }
        Ok(())
    }

    /// Same as [`AuditorClient::health_check`]
    pub async fn health_check(&self) -> bool {
        self.client.health_check().await
    }

    /// Push a record to the Auditor instance.
    ///
    /// # Errors
    ///
    /// * [`ClientError::DatabaseError`] - If there was an error inserting into the database
    #[tracing::instrument(
        name = "Pushing record to client send queue.",
        skip(self, record),
        fields(record_id = %record.record_id)
    )]
    pub async fn add(&self, record: &RecordAdd) -> Result<(), ClientError> {
        self.database.insert(record).await?;
        Ok(())
    }

    /// Push multiple records to the Auditor instance as a vec.
    ///
    /// # Errors
    ///
    /// * [`ClientError::DatabaseError`] - If there was an error inserting into the database
    #[tracing::instrument(
        name = "Pushing multiple records to client send queue.",
        skip(self, records)
    )]
    pub async fn bulk_insert(&self, records: &[RecordAdd]) -> Result<(), ClientError> {
        self.database.insert_many(records).await?;
        Ok(())
    }

    /// Update an existing record in the Auditor instance.
    ///
    /// # Errors
    ///
    /// * [`ClientError::DatabaseError`] - If there was an error inserting into the database
    #[tracing::instrument(
        name = "Pushing record update to client send queue.",
        skip(self, record),
        fields(record_id = %record.record_id)
    )]
    pub async fn update(&self, record: &RecordUpdate) -> Result<(), ClientError> {
        self.database.update(record).await?;
        Ok(())
    }

    /// Same as [`AuditorClient::get`]
    pub async fn get(&self) -> Result<Vec<Record>, ClientError> {
        self.client.get().await
    }

    /// Same as [`AuditorClient::advanced_query`]
    pub async fn advanced_query(&self, query_string: String) -> Result<Vec<Record>, ClientError> {
        self.client.advanced_query(query_string).await
    }

    /// Same as [`AuditorClient::get_single_record`]
    pub async fn get_single_record(&self, record_id: String) -> Result<Record, ClientError> {
        self.client.get_single_record(record_id).await
    }
}

// There is no async drop, so error messages are the best we can do here
impl std::ops::Drop for QueuedAuditorClient {
    fn drop(&mut self) {
        if Arc::strong_count(&self.task_handle) > 1 {
            return;
        }
        if self.shutdown_tx.lock().unwrap().is_some() || self.task_handle.lock().unwrap().is_some()
        {
            tracing::error!("Programming error: QueuedAuditorClient was not stopped");
        }
    }
}

/// The `AuditorClientBlocking` handles the interaction with the Auditor instances and allows one to add
/// records to the database, update records in the database and retrieve the records from the
/// database. In contrast to [`AuditorClient`], no async runtime is needed here.
///
/// It is constructed using the [`AuditorClientBuilder`].
#[derive(Clone)]
pub struct AuditorClientBlocking {
    address: String,
    client: reqwest::blocking::Client,
}

impl AuditorClientBlocking {
    /// Returns ``true`` if the Auditor instance is healthy, ``false`` otherwise.
    #[tracing::instrument(name = "Checking health of AUDITOR server.", skip(self))]
    pub fn health_check(&self) -> bool {
        match self
            .client
            .get(format!("{}/health_check", &self.address))
            .send()
        {
            Ok(s) => s.error_for_status().is_ok(),
            Err(_) => false,
        }
    }

    /// Push a record to the Auditor instance.
    ///
    /// # Errors
    ///
    /// * [`ClientError::RecordExists`] - If the record already exists in the database.
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Sending a record to AUDITOR server.",
        skip(self, record),
        fields(record_id = %record.record_id)
    )]
    pub fn add(&self, record: &RecordAdd) -> Result<(), ClientError> {
        let response = self
            .client
            .post(format!("{}/record", &self.address))
            .header("Content-Type", "application/json")
            .json(record)
            .send()?;

        if response.text()? == ERR_RECORD_EXISTS {
            Err(ClientError::RecordExists)
        } else {
            Ok(())
        }
    }

    /// Push multiple records to the Auditor instance as vec.
    ///
    /// # Errors
    ///
    /// * [`ClientError::RecordExists`] - If the record already exists in the database.
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Sending multiple records to AUDITOR server.",
        skip(self, records)
    )]
    pub fn bulk_insert(&self, records: &Vec<RecordAdd>) -> Result<(), ClientError> {
        let response = self
            .client
            .post(format!("{}/records", &self.address))
            .header("Content-Type", "application/json")
            .json(records)
            .send()?;

        if response.text()? == ERR_RECORD_EXISTS {
            Err(ClientError::RecordExists)
        } else {
            Ok(())
        }
    }
    /// Update an existing record in the Auditor instance.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Sending a record update to AUDITOR server.",
        skip(self, record),
        fields(record_id = %record.record_id)
    )]
    pub fn update(&self, record: &RecordUpdate) -> Result<(), ClientError> {
        self.client
            .put(format!("{}/record", &self.address))
            .header("Content-Type", "application/json")
            .json(record)
            .send()?
            .error_for_status()?;
        Ok(())
    }

    /// Gets all records from the Auditors database.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(name = "Getting all records from AUDITOR server.", skip(self))]
    pub fn get(&self) -> Result<Vec<Record>, ClientError> {
        let response = self
            .client
            .get(format!("{}/records", &self.address))
            .send()?
            .error_for_status()?;
        let reader = BufReader::new(response);
        let stream = Deserializer::from_reader(reader).into_iter::<Record>();
        let records: Vec<Record> = stream.filter_map(|result| result.ok()).collect();

        Ok(records)
    }

    /// Get all records in the database with a started timestamp after ``since``.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Getting all records started since a given date from AUDITOR server.",
        skip(self),
        fields(started_since = %since)
    )]
    #[deprecated(since = "0.4.0", note = "please use `advanced_query` instead")]
    pub fn get_started_since(&self, since: &DateTime<Utc>) -> Result<Vec<Record>, ClientError> {
        dbg!(since.to_rfc3339());
        let since_str = since.to_rfc3339();
        let encoded_since = encode(&since_str);

        let response = self
            .client
            .get(format!(
                "{}/records?start_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()?
            .error_for_status()?;

        let reader = BufReader::new(response);
        let stream = Deserializer::from_reader(reader).into_iter::<Record>();
        let records: Vec<Record> = stream.filter_map(|result| result.ok()).collect();

        Ok(records)
    }

    /// Get all records in the database with a stopped timestamp after ``since``.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Getting all records stopped since a given date from AUDITOR server.",
        skip(self),
        fields(started_since = %since)
    )]
    #[deprecated(since = "0.4.0", note = "please use `advanced_query` instead")]
    pub fn get_stopped_since(&self, since: &DateTime<Utc>) -> Result<Vec<Record>, ClientError> {
        let since_str = since.to_rfc3339();
        let encoded_since = encode(&since_str);

        let response = self
            .client
            .get(format!(
                "{}/records?stop_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()?
            .error_for_status()?;

        let reader = BufReader::new(response);
        let stream = Deserializer::from_reader(reader).into_iter::<Record>();
        let records: Vec<Record> = stream.filter_map(|result| result.ok()).collect();

        Ok(records)
    }

    /// Get records from AUDITOR server using custom filters.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    pub fn advanced_query(&self, query_params: String) -> Result<Vec<Record>, ClientError> {
        let response = self
            .client
            .get(format!("{}/records?{}", &self.address, query_params))
            .send()?
            .error_for_status()?;

        let reader = BufReader::new(response);
        let stream = Deserializer::from_reader(reader).into_iter::<Record>();
        let records: Vec<Record> = stream.filter_map(|result| result.ok()).collect();

        Ok(records)
    }

    /// Get single record from AUDITOR server using record_id.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    #[tracing::instrument(
        name = "Getting a single record from AUDITOR server using record_id",
        skip(self)
    )]
    pub fn get_single_record(&self, record_id: &str) -> Result<Record, ClientError> {
        Ok(self
            .client
            .get(format!("{}/record/{}", &self.address, record_id))
            .send()?
            .error_for_status()?
            .json()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auditor::domain::RecordTest;
    use chrono::TimeZone;
    use claim::assert_err;
    use fake::{Fake, Faker};
    use tokio::time::sleep;
    use wiremock::matchers::{any, body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn record<T: TryFrom<RecordTest>>() -> T
    where
        <T as TryFrom<RecordTest>>::Error: std::fmt::Debug,
    {
        T::try_from(Faker.fake::<RecordTest>()).unwrap()
    }

    #[tokio::test]
    async fn get_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = client.get().await.unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn blocking_get_succeeds() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = tokio::task::spawn_blocking(move || client.get().unwrap())
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn health_check_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        Mock::given(method("GET"))
            .and(path("/health_check"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert!(client.health_check().await);
    }

    #[tokio::test]
    async fn blocking_health_check_succeeds() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        Mock::given(method("GET"))
            .and(path("/health_check"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = tokio::task::spawn_blocking(move || client.health_check())
            .await
            .unwrap();

        assert!(response);
    }

    #[tokio::test]
    async fn health_check_fails_on_timeout() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .timeout(1)
            .build()
            .unwrap();

        Mock::given(method("GET"))
            .and(path("/health_check"))
            .respond_with(
                ResponseTemplate::new(200).set_delay(
                    Duration::try_seconds(180)
                        .expect("This should never fail")
                        .to_std()
                        .expect("This should never fail"),
                ),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        assert!(!client.health_check().await);
    }

    #[tokio::test]
    async fn blocking_health_check_fails_on_timeout() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .timeout(1)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        Mock::given(method("GET"))
            .and(path("/health_check"))
            .respond_with(
                ResponseTemplate::new(200).set_delay(
                    Duration::try_seconds(180)
                        .expect("This should never fail")
                        .to_std()
                        .expect("This should never fail"),
                ),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = tokio::task::spawn_blocking(move || client.health_check())
            .await
            .unwrap();

        assert!(!response);
    }

    #[tokio::test]
    async fn health_check_fails_on_500() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .timeout(1)
            .build()
            .unwrap();

        Mock::given(method("GET"))
            .and(path("/health_check"))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert!(!client.health_check().await);
    }

    #[tokio::test]
    async fn blocking_health_check_fails_on_500() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .timeout(1)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        Mock::given(method("GET"))
            .and(path("/health_check"))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = tokio::task::spawn_blocking(move || client.health_check())
            .await
            .unwrap();

        assert!(!response);
    }

    #[tokio::test]
    async fn add_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let record: RecordAdd = record();

        Mock::given(method("POST"))
            .and(path("/record"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(&record))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _res = client.add(&record).await;
    }

    // ATM a send is triggered on creation of `QueuedAuditorClient`,
    // so we don't *need* waits as long as `QueuedAuditorClient::stop` is called.
    // This is however highly implementation specific (number of awaits in each
    // code path and usage of tokios `rt` runtime).
    // So we set a low `send_interval` and wait after client.add.
    // Same is true for the other queued tests.
    #[tokio::test]
    async fn queued_add_succeeds() {
        let mock_server = MockServer::start().await;
        let mut client_builder = AuditorClientBuilder::new().connection_string(&mock_server.uri());
        client_builder.send_interval = chrono::Duration::try_milliseconds(50).unwrap();
        let mut client = client_builder.build_queued().await.unwrap();

        let record: RecordAdd = record();

        Mock::given(method("POST"))
            .and(path("/record"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(&record))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _res = client.add(&record).await;
        sleep(std::time::Duration::from_millis(100)).await;
        client.stop().await.unwrap();
    }

    #[tokio::test]
    async fn blocking_add_succeeds() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        let record: RecordAdd = record();

        Mock::given(method("POST"))
            .and(path("/record"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(&record))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _res = tokio::task::spawn_blocking(move || client.add(&record))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn add_fails_on_existing_record() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let record: RecordAdd = record();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500).set_body_string(ERR_RECORD_EXISTS))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert_err!(client.add(&record).await);
    }

    #[tokio::test]
    async fn blocking_add_fails_on_existing_record() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        let record: RecordAdd = record();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500).set_body_string(ERR_RECORD_EXISTS))
            .expect(1)
            .mount(&mock_server)
            .await;

        let res = tokio::task::spawn_blocking(move || client.add(&record))
            .await
            .unwrap();
        assert_err!(res);
    }

    #[tokio::test]
    async fn update_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let record: RecordUpdate = record();

        Mock::given(method("PUT"))
            .and(path("/record"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(&record))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _res = client.update(&record).await;
    }

    #[tokio::test]
    async fn queued_update_succeeds() {
        let mock_server = MockServer::start().await;
        let mut client_builder = AuditorClientBuilder::new().connection_string(&mock_server.uri());
        client_builder.send_interval = chrono::Duration::try_milliseconds(50).unwrap();
        let mut client = client_builder.build_queued().await.unwrap();

        let record: RecordUpdate = record();

        Mock::given(method("PUT"))
            .and(path("/record"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(&record))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _res = client.update(&record).await;
        sleep(std::time::Duration::from_millis(100)).await;
        client.stop().await.unwrap();
    }

    #[tokio::test]
    async fn blocking_update_succeeds() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        let record: RecordUpdate = record();

        Mock::given(method("PUT"))
            .and(path("/record"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(&record))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _res = tokio::task::spawn_blocking(move || client.update(&record))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn update_fails_on_500() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let record: RecordUpdate = record();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert_err!(client.update(&record).await);
    }

    #[tokio::test]
    async fn blocking_update_fails_on_500() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        let record: RecordUpdate = record();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let res = tokio::task::spawn_blocking(move || client.update(&record))
            .await
            .unwrap();
        assert_err!(res);
    }

    #[tokio::test]
    async fn get_advanced_queries_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("start_time[gte]", "2022-08-03T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let datetime_utc = Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap();
        let response = QueryBuilder::new()
            .with_start_time(Operator::default().gte(datetime_utc.into()))
            .get(client)
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_record_query_with_start_time_and_stop_time_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("start_time[gte]", "2022-08-03T09:47:00+00:00"))
            .and(query_param("stop_time[gte]", "2022-08-03T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let datetime_utc = Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap();
        let response = QueryBuilder::new()
            .with_start_time(Operator::default().gte(datetime_utc.into()))
            .with_stop_time(Operator::default().gte(datetime_utc.into()))
            .get(client)
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_record_query_with_start_time_gte_and_start_time_lte_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("start_time[gte]", "2022-08-03T09:47:00+00:00"))
            .and(query_param("start_time[lte]", "2022-08-04T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let datetime_utc_gte = Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap();
        let datetime_utc_lte = Utc.with_ymd_and_hms(2022, 8, 4, 9, 47, 0).unwrap();
        let response = QueryBuilder::new()
            .with_start_time(
                Operator::default()
                    .gte(datetime_utc_gte.into())
                    .lte(datetime_utc_lte.into()),
            )
            .get(client)
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_record_query_with_start_time_gte_and_start_time_lte_runtime_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("start_time[gte]", "2022-08-03T09:47:00+00:00"))
            .and(query_param("start_time[lte]", "2022-08-04T09:47:00+00:00"))
            .and(query_param("runtime[gte]", "100000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let datetime_utc_gte = Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap();
        let datetime_utc_lte = Utc.with_ymd_and_hms(2022, 8, 4, 9, 47, 0).unwrap();
        let runtime: u64 = 100000;
        let response = QueryBuilder::new()
            .with_start_time(
                Operator::default()
                    .gte(datetime_utc_gte.into())
                    .lte(datetime_utc_lte.into()),
            )
            .with_runtime(Operator::default().gte(runtime.into()))
            .get(client)
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_record_query_with_start_time_stop_time_and_runtime_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("start_time[gte]", "2022-08-03T09:47:00+00:00"))
            .and(query_param("start_time[lte]", "2022-08-04T09:47:00+00:00"))
            .and(query_param("runtime[gte]", "100000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let datetime_utc_gte = Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap();
        let datetime_utc_lte = Utc.with_ymd_and_hms(2022, 8, 4, 9, 47, 0).unwrap();
        let runtime_gte: u64 = 100000;
        let runtime_lte: u64 = 200000;
        let response = QueryBuilder::new()
            .with_start_time(
                Operator::default()
                    .gte(datetime_utc_gte.into())
                    .lte(datetime_utc_lte.into()),
            )
            .with_stop_time(
                Operator::default()
                    .gte(datetime_utc_gte.into())
                    .lte(datetime_utc_lte.into()),
            )
            .with_runtime(
                Operator::default()
                    .gte(runtime_gte.into())
                    .lte(runtime_lte.into()),
            )
            .get(client)
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_advanced_queries_fails_on_500() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let datetime_utc_gte = Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap();

        assert_err!(
            QueryBuilder::new()
                .with_stop_time(Operator::default().gte(datetime_utc_gte.into()))
                .get(client)
                .await
        );
    }

    #[tokio::test]
    async fn get_meta_queries_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("meta[site_id][c][0]", "group_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = QueryBuilder::new()
            .with_meta_query(MetaQuery::new().meta_operator(
                "site_id".to_string(),
                MetaOperator::default().contains(vec!["group_1".to_string()]),
            ))
            .get(client)
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_meta_queries_and_start_time_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("meta[site_id][c][0]", "group_1"))
            .and(query_param("start_time[lte]", "2022-08-04T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let datetime_utc_lte = Utc.with_ymd_and_hms(2022, 8, 4, 9, 47, 0).unwrap();
        let response = QueryBuilder::new()
            .with_meta_query(MetaQuery::new().meta_operator(
                "site_id".to_string(),
                MetaOperator::default().contains(vec!["group_1".to_string()]),
            ))
            .with_start_time(Operator::default().lte(datetime_utc_lte.into()))
            .get(client)
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_component_queries_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("component[cpu][equals]", "4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let count: u8 = 4;
        let response =
            QueryBuilder::new()
                .with_component_query(ComponentQuery::new().component_operator(
                    "cpu".to_string(),
                    Operator::default().equals(count.into()),
                ))
                .get(client)
                .await
                .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn blocking_advanced_queries_succeeds() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("stop_time[gte]", "2022-08-03T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let datetime_utc = Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap();
        let query_string = QueryBuilder::new()
            .with_stop_time(Operator::default().gte(datetime_utc.into()))
            .build();

        let response = tokio::task::spawn_blocking(move || client.advanced_query(query_string))
            .await
            .unwrap()
            .unwrap();

        println!("{:?}", &response);
        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_sort_by_query_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("sort_by[asc]", "start_time"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = QueryBuilder::new()
            .sort_by(SortBy::new().ascending("start_time".to_string()))
            .get(client)
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn limit_get_query_records_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("limit", "500"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let number: u64 = 500;
        let response = QueryBuilder::new()
            .sort_by(SortBy::new().ascending("start_time".to_string()))
            .limit(number)
            .get(client)
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_exact_record_using_record_id_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/records"))
            .and(query_param("record_id", "r1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = QueryBuilder::new()
            .with_record_id("r1".to_string())
            .get(client)
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_single_record_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let record_id: &str = "r3";

        let body: Record = record();

        Mock::given(method("GET"))
            .and(path("/record/r3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = client
            .get_single_record(record_id.to_string())
            .await
            .unwrap();

        assert_eq!(body, response)
    }

    #[tokio::test]
    async fn blocking_get_single_record_succeeds() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        let record_id: &str = "r3";

        let body: Record = record();

        Mock::given(method("GET"))
            .and(path("/record/r3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response =
            tokio::task::spawn_blocking(move || client.get_single_record(record_id).unwrap())
                .await
                .unwrap();

        assert_eq!(body, response)
    }

    #[tokio::test]
    async fn get_single_record_fails_on_500() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let record_id: &str = "r3";

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert_err!(client.get_single_record(record_id.to_string()).await);
    }

    #[tokio::test]
    async fn blocking_get_single_record_fails_on_500() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        let record_id: &str = "r3";

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let res = tokio::task::spawn_blocking(move || client.get_single_record(record_id))
            .await
            .unwrap();
        assert_err!(res);
    }

    #[tokio::test]
    async fn bulk_insert_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let records: Vec<RecordAdd> = (0..10).map(|_| record()).collect();

        Mock::given(method("POST"))
            .and(path("/records"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(&records))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _res = client.bulk_insert(&records).await;
    }

    /*
    #[tokio::test]
    async fn queued_bulk_insert_succeeds() {
        let mock_server = MockServer::start().await;
        let mut client_builder = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri());
        client_builder.send_interval = chrono::Duration::try_milliseconds(50).unwrap();
        let mut client = client_builder
            .build_queued()
            .await
            .unwrap();

        let records: Vec<RecordAdd> = (0..10).map(|_| record()).collect();

        Mock::given(method("POST"))
            .and(path("/records"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(&records))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _res = client.bulk_insert(&records).await;
        sleep(std::time::Duration::from_millis(100)).await;
        client.stop().await.unwrap();
    }
    */

    #[tokio::test]
    async fn queued_client_stop_raises_error() {
        let mut client = AuditorClientBuilder::new()
            .address(&"localhost", 8000)
            .build_queued()
            .await
            .unwrap();

        client.stop().await.unwrap();
        assert_err!(client.stop().await);
    }

    #[tokio::test]
    async fn blocking_bulk_insert_succeeds() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        let records: Vec<RecordAdd> = (0..10).map(|_| record()).collect();

        Mock::given(method("POST"))
            .and(path("/records"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(&records))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _res = tokio::task::spawn_blocking(move || client.bulk_insert(&records))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn bulk_insert_fails_on_existing_record() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let records: Vec<RecordAdd> = (0..10).map(|_| record()).collect();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500).set_body_string(ERR_RECORD_EXISTS))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert_err!(client.bulk_insert(&records).await);
    }

    #[tokio::test]
    async fn blocking_bulk_insert_fails_on_existing_record() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = tokio::task::spawn_blocking(move || {
            AuditorClientBuilder::new()
                .connection_string(&uri)
                .build_blocking()
                .unwrap()
        })
        .await
        .unwrap();

        let records: Vec<RecordAdd> = (0..10).map(|_| record()).collect();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500).set_body_string(ERR_RECORD_EXISTS))
            .expect(1)
            .mount(&mock_server)
            .await;

        let res = tokio::task::spawn_blocking(move || client.bulk_insert(&records))
            .await
            .unwrap();
        assert_err!(res);
    }
}
