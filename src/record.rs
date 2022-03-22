use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgHasArrayType;

#[derive(Debug, PartialEq, Serialize, Deserialize, sqlx::Type, Clone)]
#[sqlx(type_name = "component")]
pub struct Component {
    pub name: String,
    pub amount: i64,
    pub factor: f64,
}

impl PgHasArrayType for Component {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_component")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordAdd {
    pub record_id: String,
    pub site_id: String,
    pub user_id: String,
    pub group_id: String,
    pub components: Vec<Component>,
    pub start_time: DateTime<Utc>,
    pub stop_time: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordUpdate {
    pub record_id: String,
    pub site_id: String,
    pub user_id: String,
    pub group_id: String,
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
