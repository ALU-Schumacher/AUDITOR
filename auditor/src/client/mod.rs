// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::{
    constants::ERR_RECORD_EXISTS,
    domain::{Record, RecordAdd, RecordUpdate},
};
use anyhow::Error;
use chrono::{DateTime, Duration, Utc};
use reqwest;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ClientError {
    RecordExists,
    ReqwestError(reqwest::Error),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ClientError::RecordExists => ERR_RECORD_EXISTS.to_string(),
                ClientError::ReqwestError(e) => format!("Reqwest Error: {e}"),
            }
        )
    }
}

#[derive(Clone)]
pub struct AuditorClientBuilder {
    address: String,
    timeout: Duration,
}

impl AuditorClientBuilder {
    pub fn new() -> AuditorClientBuilder {
        AuditorClientBuilder {
            address: "127.0.0.1:8080".into(),
            timeout: Duration::seconds(30),
        }
    }

    #[must_use]
    pub fn address<T: AsRef<str>>(mut self, address: &T, port: u16) -> Self {
        self.address = format!("http://{}:{}", address.as_ref(), port);
        self
    }

    #[must_use]
    pub fn connection_string<T: AsRef<str>>(mut self, connection_string: &T) -> Self {
        self.address = connection_string.as_ref().into();
        self
    }

    #[must_use]
    pub fn timeout(mut self, timeout: i64) -> Self {
        self.timeout = Duration::seconds(timeout);
        self
    }

    pub fn build(self) -> Result<AuditorClient, Error> {
        Ok(AuditorClient {
            address: self.address,
            client: reqwest::ClientBuilder::new()
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

#[derive(Clone)]
pub struct AuditorClient {
    address: String,
    client: reqwest::Client,
}

impl AuditorClient {
    pub fn new<T: AsRef<str>>(address: &T, port: u16) -> Result<AuditorClient, reqwest::Error> {
        Ok(AuditorClient {
            address: format!("http://{}:{}", address.as_ref(), port),
            client: reqwest::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .build()?,
        })
    }

    pub fn from_connection_string<T: AsRef<str>>(
        connection_string: &T,
    ) -> Result<AuditorClient, reqwest::Error> {
        Ok(AuditorClient {
            address: connection_string.as_ref().into(),
            client: reqwest::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .build()?,
        })
    }

    #[tracing::instrument(name = "Checking health of AUDITOR server.", skip(self))]
    pub async fn health_check(&self) -> bool {
        match self
            .client
            .get(&format!("{}/health_check", &self.address))
            .send()
            .await
        {
            Ok(s) => matches!(s.error_for_status(), Ok(_)),
            Err(_) => false,
        }
    }

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
            .await
            .map_err(ClientError::ReqwestError)?;

        if response.text().await.map_err(ClientError::ReqwestError)? == ERR_RECORD_EXISTS {
            Err(ClientError::RecordExists)
        } else {
            Ok(())
        }
    }

    #[tracing::instrument(
        name = "Sending a record update to AUDITOR server.",
        skip(self, record),
        fields(record_id = %record.record_id)
    )]
    pub async fn update(&self, record: &RecordUpdate) -> Result<(), reqwest::Error> {
        self.client
            .post(&format!("{}/update", &self.address))
            .header("Content-Type", "application/json")
            .json(record)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    #[tracing::instrument(name = "Getting all records from AUDITOR server.", skip(self))]
    pub async fn get(&self) -> Result<Vec<Record>, reqwest::Error> {
        self.client
            .get(&format!("{}/get", &self.address))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    #[tracing::instrument(
        name = "Getting all records started since a given date from AUDITOR server.",
        skip(self),
        fields(started_since = %since)
    )]
    pub async fn get_started_since(
        &self,
        since: &DateTime<Utc>,
    ) -> Result<Vec<Record>, reqwest::Error> {
        dbg!(since.to_rfc3339());
        self.client
            .get(&format!(
                "{}/get/started/since/{}",
                &self.address,
                since.to_rfc3339()
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    #[tracing::instrument(
        name = "Getting all records stopped since a given date from AUDITOR server.",
        skip(self),
        fields(started_since = %since)
    )]
    pub async fn get_stopped_since(
        &self,
        since: &DateTime<Utc>,
    ) -> Result<Vec<Record>, reqwest::Error> {
        self.client
            .get(&format!(
                "{}/get/stopped/since/{}",
                &self.address,
                since.to_rfc3339()
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
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
        let client = AuditorClient::from_connection_string(&mock_server.uri()).unwrap();

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
            .zip(body.into_iter())
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn health_check_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClient::from_connection_string(&mock_server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path("/health_check"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert!(client.health_check().await);
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
    async fn add_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClient::from_connection_string(&mock_server.uri()).unwrap();

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
    async fn add_fails_on_existing_record() {
        let mock_server = MockServer::start().await;
        let client = AuditorClient::from_connection_string(&mock_server.uri()).unwrap();

        let record: RecordAdd = record();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500).set_body_string(ERR_RECORD_EXISTS))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert_err!(client.add(&record).await);
    }

    #[tokio::test]
    async fn update_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClient::from_connection_string(&mock_server.uri()).unwrap();

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
    async fn update_fails_on_500() {
        let mock_server = MockServer::start().await;
        let client = AuditorClient::from_connection_string(&mock_server.uri()).unwrap();

        let record: RecordUpdate = record();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert_err!(client.update(&record).await);
    }

    #[tokio::test]
    async fn get_started_since_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClient::from_connection_string(&mock_server.uri()).unwrap();

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
            .zip(body.into_iter())
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_started_since_fails_on_500() {
        let mock_server = MockServer::start().await;
        let client = AuditorClient::from_connection_string(&mock_server.uri()).unwrap();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        assert_err!(
            client
                .get_started_since(&Utc.with_ymd_and_hms(2022, 8, 3, 9, 47, 0).unwrap())
                .await
        );
    }

    #[tokio::test]
    async fn get_stopped_since_succeeds() {
        let mock_server = MockServer::start().await;
        let client = AuditorClient::from_connection_string(&mock_server.uri()).unwrap();

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
            .zip(body.into_iter())
            .map(|(rr, br)| assert_eq!(rr, br))
            .count();
    }

    #[tokio::test]
    async fn get_stopped_since_fails_on_500() {
        let mock_server = MockServer::start().await;
        let client = AuditorClient::from_connection_string(&mock_server.uri()).unwrap();

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
}
