// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! This module provides a client to interact with an Auditor instance.

use crate::{
    constants::{ERR_INVALID_TIMEOUT, ERR_RECORD_EXISTS},
    domain::{Record, RecordAdd, RecordUpdate},
};
use chrono::{DateTime, Duration, Utc};
use reqwest;
use serde::Serialize;
use std::collections::HashMap;
use urlencoding::encode;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ClientError {
    RecordExists,
    InvalidTimeout,
    ReqwestError(reqwest::Error),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ClientError::RecordExists => ERR_RECORD_EXISTS.to_string(),
                ClientError::InvalidTimeout => ERR_INVALID_TIMEOUT.to_string(),
                ClientError::ReqwestError(e) => format!("Reqwest Error: {e}"),
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
        ClientError::InvalidTimeout
    }
}

/// The `AuditorClientBuilder` is used to build an instance of
/// either [`AuditorClient`] or [`AuditorClientBlocking`].
///
/// # Examples
///
/// Using the `address` and `port` of the Auditor instance:
///
/// ```
/// # use auditor::client::{AuditorClientBuilder, ClientError};
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
/// # use auditor::client::{AuditorClientBuilder, ClientError};
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
    timeout: Duration,
}

impl AuditorClientBuilder {
    /// Constructor.
    pub fn new() -> AuditorClientBuilder {
        AuditorClientBuilder {
            address: "http://127.0.0.1:8080".into(),
            timeout: Duration::seconds(30),
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
        self.address = format!("http://{}:{}", address.as_ref(), port);
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
        self.timeout = Duration::seconds(timeout);
        self
    }

    /// Build an [`AuditorClient`] from `AuditorClientBuilder`.
    ///
    /// # Errors
    ///
    /// * [`ClientError::InvalidTimeout`] - If the timeout duration is less than zero.
    /// * [`ClientError::ReqwestError`] - If there was an error building the HTTP client.
    pub fn build(self) -> Result<AuditorClient, ClientError> {
        Ok(AuditorClient {
            address: self.address,
            client: reqwest::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .timeout(self.timeout.to_std()?)
                .build()?,
        })
    }

    /// Build an [`AuditorClientBlocking`] from `AuditorClientBuilder`.
    ///
    /// # Errors
    ///
    /// * [`ClientError::InvalidTimeout`] - If the timeout duration is less than zero.
    /// * [`ClientError::ReqwestError`] - If there was an error building the HTTP client.
    ///
    /// # Panics
    ///
    /// This method panics if it is called from an async runtime.
    pub fn build_blocking(self) -> Result<AuditorClientBlocking, ClientError> {
        Ok(AuditorClientBlocking {
            address: self.address,
            client: reqwest::blocking::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .timeout(self.timeout.to_std()?)
                .build()?,
        })
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

/// Implementations of conversion traits for the `Value` enum.

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
/// use auditor::client::{QueryBuilder, Operator, MetaQuery, ComponentQuery, SortBy};
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
    pub c: Option<String>,
    /// `does not contains` - Specifies if the meta key does not contains the value.
    pub dnc: Option<String>,
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
    pub fn contains(mut self, c: String) -> Self {
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
    pub fn does_not_contains(mut self, dnc: String) -> Self {
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
            .get(&format!("{}/health_check", &self.address))
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
        fields(record_id = %record.record_id)
    )]
    pub async fn add(&self, record: &RecordAdd) -> Result<(), ClientError> {
        let response = self
            .client
            .post(&format!("{}/record", &self.address))
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
            .post(&format!("{}/records", &self.address))
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
            .put(&format!("{}/record", &self.address))
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
        Ok(self
            .client
            .get(&format!("{}/records", &self.address))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
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
        Ok(self
            .client
            .get(&format!(
                "{}/records?start_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
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
        Ok(self
            .client
            .get(&format!(
                "{}/records?stop_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
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
        Ok(self
            .client
            .get(&format!("{}/records?{}", &self.address, query_string))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
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
            .get(&format!("{}/record/{}", &self.address, record_id))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
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
        Ok(self
            .client
            .get(format!("{}/records", &self.address))
            .send()?
            .error_for_status()?
            .json()?)
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
        Ok(self
            .client
            .get(format!(
                "{}/records?start_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()?
            .error_for_status()?
            .json()?)
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
        Ok(self
            .client
            .get(format!(
                "{}/records?stop_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()?
            .error_for_status()?
            .json()?)
    }

    /// Get records from AUDITOR server using custom filters.
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    pub fn advanced_query(&self, query_params: String) -> Result<Vec<Record>, ClientError> {
        Ok(self
            .client
            .get(format!("{}/records?{}", &self.address, query_params))
            .send()?
            .error_for_status()?
            .json()?)
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
    use crate::domain::RecordTest;
    use chrono::TimeZone;
    use claim::assert_err;
    use fake::{Fake, Faker};
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
                ResponseTemplate::new(200).set_delay(Duration::seconds(180).to_std().unwrap()),
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
                ResponseTemplate::new(200).set_delay(Duration::seconds(180).to_std().unwrap()),
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
            .and(query_param("meta[site_id][c]", "group_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = QueryBuilder::new()
            .with_meta_query(MetaQuery::new().meta_operator(
                "site_id".to_string(),
                MetaOperator::default().contains("group_1".to_string()),
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
            .and(query_param("meta[site_id][c]", "group_1"))
            .and(query_param("start_time[lte]", "2022-08-04T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let datetime_utc_lte = Utc.with_ymd_and_hms(2022, 8, 4, 9, 47, 0).unwrap();
        let response = QueryBuilder::new()
            .with_meta_query(MetaQuery::new().meta_operator(
                "site_id".to_string(),
                MetaOperator::default().contains("group_1".to_string()),
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
