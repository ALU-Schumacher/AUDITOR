// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

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
//! The [`AuditorClientBuilder`](`crate::client::AuditorClientBuilder`) is used to build an [`AuditorClient`](`crate::client::AuditorClient`) object
//! that can be used for interacting with Auditor.
//!
//! ```
//! use auditor::client::AuditorClientBuilder;
//! # use auditor::client::ClientError;
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
//! ## Pushing one record to Auditor
//!
//! Assuming that a record and a client were already created,
//! the record can be pushed to Auditor with
//!
//! ```no_run
//! # use auditor::client::{AuditorClientBuilder, ClientError};
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
//! # use auditor::client::{AuditorClientBuilder, ClientError};
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
//! # use auditor::client::{AuditorClientBuilder, ClientError};
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
//! # use auditor::client::{AuditorClientBuilder, ClientError};
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
//! # use auditor::client::{AuditorClientBuilder, ClientError};
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
//! - `dnc` (does not contains)
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
//!| `meta`       | Meta information (<meta_key>, MetaOperator(<meta_value>))              | `d`, `dnc`                             | `meta[<meta_key>][c]=<meta_value>`         |
//!| `component`  | Component identifier (<component_name>, Operator(<component_amount>))  | `gt`, `gte`, `lt`, `lte`, `equals`     | `component[<component_name>][gt]=<amount>` |
//!| `sort_by`    | Sort query results (SortBy(<column_name>))                             | `asc`, `desc`                          | `sort_by[desc]=<column_name>`              |
//!| `limit`      | limit query records (number)                                           |                                        | `limit=5000`                               |
//!
//! Meta field can be used to query records by specifying the meta key and [`MetaOperator`](`crate::client::MetaOperator`)  must be used
//! to specify meta values. The [`MetaOperator`](`crate::client::MetaOperator`) must be used to specify whether the value is
//! contained or does not contained for the specific Metakey.
//!
//! Component field can be used to query records by specifying the component name (CPU) and ['Operator'] must be used
//! to specify the amount.
//!
//! To query records based on a range, specify the field with two operators
//! Either with gt or gte and lt or lte.
//!
//! For example, to query a records with start_time ranging between two timestamps:
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
//! Constructs an empty [`QueryBuilder`](`crate::client::QueryBuilder`) to query all records
//!
//! ```no_run
//! # use auditor::client::{QueryBuilder, AuditorClientBuilder, ClientError};
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
//! # use auditor::client::{QueryBuilder, Operator, AuditorClientBuilder, ClientError};
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
//! # use auditor::client::{QueryBuilder, Operator, AuditorClientBuilder, ClientError};
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
//! # use auditor::client::{QueryBuilder, Operator, MetaQuery, MetaOperator, AuditorClientBuilder,
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
//!             MetaOperator::default().contains("site1".to_string()),
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
//! GET records?meta[site_id][c]=site1&start_time[lte]=datetime_utc_lte
//! ```
//!
//! ### Example 5:
//!
//! Constructs a QueryBuilder with a component query specifying an equality condition for the "CPU" field.
//!
//! ```no_run
//! use auditor::client::{QueryBuilder, Operator, ComponentQuery, AuditorClientBuilder, ClientError};
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
//! use auditor::client::{QueryBuilder, AuditorClientBuilder, ClientError, SortBy};
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
//! use auditor::client::{QueryBuilder, AuditorClientBuilder, ClientError};
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
//! # use auditor::client::{AuditorClientBuilder, ClientError};
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
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "server")]
pub mod configuration;
pub mod constants;
pub mod domain;
pub mod error;
#[cfg(feature = "server")]
pub mod metrics;
#[macro_use]
mod macros;
#[cfg(feature = "server")]
pub mod routes;
#[cfg(feature = "server")]
pub mod startup;
pub mod telemetry;
