use crate::domain::{Record, RecordAdd};
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

    pub async fn get(&self) -> Result<Vec<Record>, reqwest::Error> {
        self.client
            .get(&format!("{}/get", &self.address))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn add(_record: RecordAdd) -> Result<(), std::io::Error> {
        todo!()
    }
}
