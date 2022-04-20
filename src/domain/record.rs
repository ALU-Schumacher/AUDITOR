//! Record related types used for deserializing HTTP requests and serializing HTTP responses.

use super::{Component, ComponentTest, ValidName};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordAdd {
    pub record_id: ValidName,
    pub site_id: ValidName,
    pub user_id: ValidName,
    pub group_id: ValidName,
    pub components: Vec<Component>,
    pub start_time: DateTime<Utc>,
    pub stop_time: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordUpdate {
    pub record_id: ValidName,
    pub site_id: ValidName,
    pub user_id: ValidName,
    pub group_id: ValidName,
    pub components: Vec<Component>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Record {
    pub record_id: String,
    pub site_id: Option<String>,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub components: Option<Vec<Component>>,
    pub start_time: DateTime<Utc>,
    pub stop_time: Option<DateTime<Utc>>,
    pub runtime: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RecordTest {
    pub record_id: Option<String>,
    pub site_id: Option<String>,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub components: Option<Vec<ComponentTest>>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: Option<DateTime<Utc>>,
}

impl RecordTest {
    pub fn new() -> Self {
        RecordTest::default()
    }

    pub fn with_record_id<T: AsRef<str>>(mut self, record_id: T) -> Self {
        self.record_id = Some(record_id.as_ref().to_string());
        self
    }

    pub fn with_site_id<T: AsRef<str>>(mut self, site_id: T) -> Self {
        self.site_id = Some(site_id.as_ref().to_string());
        self
    }

    pub fn with_user_id<T: AsRef<str>>(mut self, user_id: T) -> Self {
        self.user_id = Some(user_id.as_ref().to_string());
        self
    }

    pub fn with_group_id<T: AsRef<str>>(mut self, group_id: T) -> Self {
        self.group_id = Some(group_id.as_ref().to_string());
        self
    }

    pub fn with_component<T: AsRef<str>>(mut self, name: T, amount: i64, factor: f64) -> Self {
        if self.components.is_none() {
            self.components = Some(vec![])
        }
        self.components.as_mut().unwrap().push(ComponentTest {
            name: Some(name.as_ref().to_string()),
            amount: Some(amount),
            factor: Some(factor),
        });
        self
    }

    pub fn with_start_time<T: AsRef<str>>(mut self, start_time: T) -> Self {
        self.start_time = Some(
            DateTime::parse_from_rfc3339(start_time.as_ref())
                .unwrap()
                .with_timezone(&Utc),
        );
        self
    }

    pub fn with_stop_time<T: AsRef<str>>(mut self, stop_time: T) -> Self {
        self.stop_time = Some(
            DateTime::parse_from_rfc3339(stop_time.as_ref())
                .unwrap()
                .with_timezone(&Utc),
        );
        self
    }
}
