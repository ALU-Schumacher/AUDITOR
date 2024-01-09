use crate::routes::GetFilterError;
use crate::routes::{advanced_record_filtering, get_one_record, Filters};
use actix_web::{web, HttpRequest, HttpResponse};
use sqlx;
use sqlx::PgPool;

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

    let filters: Filters = serde_qs::from_str(query_string).unwrap();

    if query_string.is_empty() {
        // This case explicitly checks if the query is empty. Then it returns all records.
        let records = advanced_record_filtering(filters, &pool)
            .await
            .map_err(GetFilterError::UnexpectedError)?;
        return Ok(HttpResponse::Ok().json(records));
    }

    if filters.is_all_none() {
        return Err(GetFilterError::UnexpectedError(anyhow::Error::msg(
            "Query is incorrect, please check the query string",
        )));
    }

    let records = advanced_record_filtering(filters, &pool)
        .await
        .map_err(GetFilterError::UnexpectedError)?;
    Ok(HttpResponse::Ok().json(records))
}

#[tracing::instrument(name = "Getting one record", skip(record_query, pool))]
pub async fn query_one_record(
    record_query: web::Path<String>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, GetFilterError> {
    let record = get_one_record(record_query.to_string(), &pool)
        .await
        .map_err(GetFilterError::UnexpectedError)?;
    Ok(HttpResponse::Ok().json(record))
}
