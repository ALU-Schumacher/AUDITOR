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
//! ## Pushing records to Auditor
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
//! ## Receiving all records started/stopped since a given timestamp
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
