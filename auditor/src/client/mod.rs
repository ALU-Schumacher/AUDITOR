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

#[derive(serde::Deserialize, Debug, Default, Clone)]
pub struct DateTimeUtcWrapper(pub DateTime<Utc>);

impl Serialize for DateTimeUtcWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_rfc3339())
    }
}

/// The 'QueryParameters' is used to build query parameters which allows to query records from
/// the database using advanced_query function.
#[derive(serde::Deserialize, serde::Serialize, Debug, Default, Clone)]
pub struct QueryParameters {
    pub start_time: Option<TimeOperator>,
    pub stop_time: Option<TimeOperator>,
    pub runtime: Option<TimeOperator>,
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub enum TimeValue {
    Datetime(DateTimeUtcWrapper),
    Runtime(u64),
}

impl Serialize for TimeValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            TimeValue::Datetime(datetime) => datetime.serialize(serializer),
            TimeValue::Runtime(runtime) => runtime.serialize(serializer),
        }
    }
}

/// The 'TimeOperator' struct is used to specify the operators on the query parameters like (gt ->
/// greater than, lt -> lesser than, gte -> greater than equals, lte -> lesser than equals)
#[derive(serde::Deserialize, serde::Serialize, Debug, Default, Clone)]
pub struct TimeOperator {
    pub gt: Option<TimeValue>,
    pub lt: Option<TimeValue>,
    pub gte: Option<TimeValue>,
    pub lte: Option<TimeValue>,
}

impl TimeOperator {
    pub fn gt(mut self, value: TimeValue) -> Self {
        self.gt = Some(value);
        self
    }

    pub fn lt(mut self, value: TimeValue) -> Self {
        self.lt = Some(value);
        self
    }

    pub fn gte(mut self, value: TimeValue) -> Self {
        self.gte = Some(value);
        self
    }

    pub fn lte(mut self, value: TimeValue) -> Self {
        self.lte = Some(value);
        self
    }
}

impl From<chrono::DateTime<Utc>> for TimeValue {
    fn from(item: chrono::DateTime<Utc>) -> Self {
        TimeValue::Datetime(DateTimeUtcWrapper(item))
    }
}

impl From<u64> for TimeValue {
    fn from(item: u64) -> Self {
        TimeValue::Runtime(item)
    }
}

/// The Builder to automatically build the 'QueryParameter' struct using builder pattern
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    pub query_params: QueryParameters,
}

impl QueryBuilder {
    pub fn new() -> Self {
        QueryBuilder {
            query_params: QueryParameters {
                start_time: None,
                stop_time: None,
                runtime: None,
            },
        }
    }

    pub fn with_start_time(mut self, time_operator: TimeOperator) -> Self {
        self.query_params.start_time = Some(time_operator);
        self
    }

    pub fn with_stop_time(mut self, time_operator: TimeOperator) -> Self {
        self.query_params.stop_time = Some(time_operator);
        self
    }

    pub fn with_runtime(mut self, time_operator: TimeOperator) -> Self {
        self.query_params.runtime = Some(time_operator);
        self
    }

    pub async fn get(&self, client: AuditorClient) -> Result<Vec<Record>, ClientError> {
        let query_string =
            serde_qs::to_string(&self.query_params).expect("Failed to serialize query parameters");
        println!("{}", &query_string);
        client.advanced_query(query_string).await
    }

    pub fn build(&self) -> String {
        serde_qs::to_string(&self.query_params).expect("Failed to serialize query parameters")
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
            .get(&format!("{}/record", &self.address))
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
                "{}/record?start_time[gte]={}",
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
    pub async fn get_stopped_since(
        &self,
        since: &DateTime<Utc>,
    ) -> Result<Vec<Record>, ClientError> {
        let since_str = since.to_rfc3339();
        let encoded_since = encode(&since_str);
        Ok(self
            .client
            .get(&format!(
                "{}/record?stop_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    #[tracing::instrument(
        name = "Getting custom record queries from AUDITOR server.",
        skip(self)
    )]
    pub async fn advanced_query(&self, query_string: String) -> Result<Vec<Record>, ClientError> {
        Ok(self
            .client
            .get(&format!("{}/record?{}", &self.address, query_string))
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
            .get(format!("{}/record", &self.address))
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
    pub fn get_started_since(&self, since: &DateTime<Utc>) -> Result<Vec<Record>, ClientError> {
        dbg!(since.to_rfc3339());
        let since_str = since.to_rfc3339();
        let encoded_since = encode(&since_str);
        Ok(self
            .client
            .get(format!(
                "{}/record?start_time[gte]={}",
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
    pub fn get_stopped_since(&self, since: &DateTime<Utc>) -> Result<Vec<Record>, ClientError> {
        let since_str = since.to_rfc3339();
        let encoded_since = encode(&since_str);
        Ok(self
            .client
            .get(format!(
                "{}/record?stop_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()?
            .error_for_status()?
            .json()?)
    }

    /// Get custom records using filters - start_time, stop_time, runtime_filters
    ///
    /// # Errors
    ///
    /// * [`ClientError::ReqwestError`] - If there was an error sending the HTTP request.
    pub async fn advanced_query(&self, query_params: String) -> Result<Vec<Record>, ClientError> {
        Ok(self
            .client
            .get(format!("{}/record?{}", &self.address, query_params))
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
            .and(path("/record"))
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
            .and(path("/record"))
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
    async fn get_started_since_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/record"))
            .and(query_param("start_time[gte]", "2022-08-03T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = client
            .get_started_since(&Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap())
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn blocking_get_started_since_succeeds() {
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
            .and(path("/record"))
            .and(query_param("start_time[gte]", "2022-08-03T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = tokio::task::spawn_blocking(move || {
            client.get_started_since(&Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap())
        })
        .await
        .unwrap()
        .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_started_since_fails_on_500() {
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

        let response = client
            .get_started_since(&Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap())
            .await;

        assert_err!(response);
    }

    #[tokio::test]
    async fn blocking_get_started_since_fails_on_500() {
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

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = tokio::task::spawn_blocking(move || {
            client.get_started_since(&Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap())
        })
        .await
        .unwrap();

        assert_err!(response);
    }

    #[tokio::test]
    async fn get_stopped_since_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClientBuilder::new()
            .connection_string(&mock_server.uri())
            .build()
            .unwrap();

        let body: Vec<Record> = vec![record()];

        Mock::given(method("GET"))
            .and(path("/record"))
            .and(query_param("stop_time[gte]", "2022-08-03T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = client
            .get_stopped_since(&Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap())
            .await
            .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn blocking_get_stopped_since_succeeds() {
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
            .and(path("/record"))
            .and(query_param("stop_time[gte]", "2022-08-03T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = tokio::task::spawn_blocking(move || {
            client.get_stopped_since(&Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap())
        })
        .await
        .unwrap()
        .unwrap();

        response
            .into_iter()
            .zip(body)
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_stopped_since_fails_on_500() {
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

        assert_err!(
            client
                .get_stopped_since(&Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap())
                .await
        );
    }

    #[tokio::test]
    async fn blocking_get_stopped_since_fails_on_500() {
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

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let response = tokio::task::spawn_blocking(move || {
            client.get_stopped_since(&Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap())
        })
        .await
        .unwrap();

        assert_err!(response);
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
            .and(path("/record"))
            .and(query_param("start_time[gte]", "2022-08-03T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let datetime_utc = Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap();
        let response = QueryBuilder::new()
            .with_start_time(TimeOperator::default().gte(datetime_utc.into()))
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
            .and(path("/record"))
            .and(query_param("start_time[gte]", "2022-08-03T09:47:00+00:00"))
            .and(query_param("stop_time[gte]", "2022-08-03T09:47:00+00:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let datetime_utc = Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap();
        let response = QueryBuilder::new()
            .with_start_time(TimeOperator::default().gte(datetime_utc.into()))
            .with_stop_time(TimeOperator::default().gte(datetime_utc.into()))
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
            .and(path("/record"))
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
                TimeOperator::default()
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
            .and(path("/record"))
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
                TimeOperator::default()
                    .gte(datetime_utc_gte.into())
                    .lte(datetime_utc_lte.into()),
            )
            .with_runtime(TimeOperator::default().gte(runtime.into()))
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
            .and(path("/record"))
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
                TimeOperator::default()
                    .gte(datetime_utc_gte.into())
                    .lte(datetime_utc_lte.into()),
            )
            .with_stop_time(
                TimeOperator::default()
                    .gte(datetime_utc_gte.into())
                    .lte(datetime_utc_lte.into()),
            )
            .with_runtime(
                TimeOperator::default()
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
                .with_stop_time(TimeOperator::default().gte(datetime_utc_gte.into()))
                .get(client)
                .await
        );
    }
}
