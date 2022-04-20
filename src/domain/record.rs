//! Record related types used for deserializing HTTP requests and serializing HTTP responses.

use super::{Component, ValidName};
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
