//! TODO: Handle failures.

use crate::domain::{Record, RecordAdd, RecordUpdate};
use reqwest;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

pub struct AuditorClient {
    address: String,
    client: reqwest::Client,
}

impl AuditorClient {
    pub fn new<T: AsRef<str>>(address: &T, port: usize) -> Result<AuditorClient, reqwest::Error> {
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

    #[tracing::instrument(name = "Getting all records from AUDITOR server.", skip(self))]
    pub async fn get(&self) -> Result<Vec<Record>, reqwest::Error> {
        self.client
            .get(&format!("{}/get", &self.address))
            .send()
            .await?
            .json()
            .await
    }

    #[tracing::instrument(
        name = "Sending a record to AUDITOR server.",
        skip(self, record),
        fields(record_id = %record.record_id)
    )]
    pub async fn add(&self, record: RecordAdd) -> Result<(), reqwest::Error> {
        self.client
            .post(&format!("{}/add", &self.address))
            .header("Content-Type", "application/json")
            .json(&record)
            .send()
            .await?;
        Ok(())
    }

    #[tracing::instrument(
        name = "Sending a record update to AUDITOR server.",
        skip(self, record),
        fields(record_id = %record.record_id)
    )]
    pub async fn update(&self, record: RecordUpdate) -> Result<(), reqwest::Error> {
        self.client
            .post(&format!("{}/update", &self.address))
            .header("Content-Type", "application/json")
            .json(&record)
            .send()
            .await?;
        Ok(())
    }
}
