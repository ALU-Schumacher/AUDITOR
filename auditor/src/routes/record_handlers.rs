use crate::routes::{advanced_record_filtering, get_one_record, Filters};
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use serde_json::json;
use sqlx::PgPool;
use thiserror::Error;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct RecordQuery {
    pub record_id: String,
}

#[tracing::instrument(name = "Getting records", skip(query, pool))]
pub async fn query_records(
    query: HttpRequest,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, GetFilterError> {
    let query_string = query.query_string();

    let filters: Filters = match serde_qs::from_str(query_string) {
        Ok(filters) => filters,
        Err(_) => return Err(GetFilterError::InvalidQuery),
    };

    if query_string.is_empty() {
        // This case explicitly checks if the query is empty. Then it returns all records.
        let records = advanced_record_filtering(filters, &pool)
            .await
            .map_err(|err| GetFilterError::UnexpectedError(err.to_string()))?;

        return Ok(HttpResponse::Ok().json(records));
    }

    if filters.is_all_none() {
        return Err(GetFilterError::InvalidQuery);
    }

    let records = advanced_record_filtering(filters, &pool)
        .await
        .map_err(|err| GetFilterError::UnexpectedError(err.to_string()))?;

    Ok(HttpResponse::Ok().json(records))
}

#[tracing::instrument(name = "Getting one record", skip(record_query, pool))]
pub async fn query_one_record(
    record_query: web::Path<String>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, GetFilterError> {
    let record = get_one_record(record_query.to_string(), &pool)
        .await
        .map_err(|err| GetFilterError::UnexpectedError(err.to_string()))?;
    Ok(HttpResponse::Ok().json(record))
}

#[derive(Debug, Error)]
pub enum GetFilterError {
    #[error("Invalid query parameters")]
    InvalidQuery,

    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
}

impl ResponseError for GetFilterError {
    fn error_response(&self) -> HttpResponse {
        match self {
            GetFilterError::InvalidQuery => {
                HttpResponse::BadRequest().json(json!({ "error": "Invalid query parameters" }))
            }
            GetFilterError::UnexpectedError(ref err) => {
                HttpResponse::InternalServerError().json(json!({ "error": err }))
            }
        }
    }
}
