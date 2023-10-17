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
            .post(&format!("{}/add", &self.address))
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
            .post(&format!("{}/update", &self.address))
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
            .get(&format!("{}/get", &self.address))
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
        Ok(self
            .client
            .get(&format!(
                "{}/get/started/since/{}",
                &self.address,
                since.to_rfc3339()
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
        Ok(self
            .client
            .get(&format!(
                "{}/get/stopped/since/{}",
                &self.address,
                since.to_rfc3339()
            ))
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
            .post(format!("{}/add", &self.address))
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
            .post(format!("{}/update", &self.address))
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
            .get(format!("{}/get", &self.address))
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
        Ok(self
            .client
            .get(format!(
                "{}/get/started/since/{}",
                &self.address,
                since.to_rfc3339()
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
        Ok(self
            .client
            .get(format!(
                "{}/get/stopped/since/{}",
                &self.address,
                since.to_rfc3339()
            ))
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
    use wiremock::matchers::{any, body_json, header, method, path};
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
            .and(path("/get"))
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
            .and(path("/get"))
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
            .and(path("/add"))
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
            .and(path("/add"))
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

        Mock::given(method("POST"))
            .and(path("/update"))
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

        Mock::given(method("POST"))
            .and(path("/update"))
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
            .and(path("/get/started/since/2022-08-03T09:47:00+00:00"))
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
            .and(path("/get/started/since/2022-08-03T09:47:00+00:00"))
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
            .and(path("/get/stopped/since/2022-08-03T09:47:00+00:00"))
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
            .and(path("/get/stopped/since/2022-08-03T09:47:00+00:00"))
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
}
