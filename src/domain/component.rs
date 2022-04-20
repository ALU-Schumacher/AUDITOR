use super::ValidName;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgHasArrayType;

#[derive(Debug, PartialEq, Serialize, Deserialize, sqlx::Type, Clone)]
#[sqlx(type_name = "component")]
pub struct Component {
    pub name: ValidName,
    pub amount: i64,
    pub factor: f64,
}

impl PgHasArrayType for Component {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_component")
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentTest {
    pub name: Option<String>,
    pub amount: Option<i64>,
    pub factor: Option<f64>,
}
