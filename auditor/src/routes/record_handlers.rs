use crate::routes::GetFilterError;
use crate::routes::{advanced_record_filtering, get_one_record, Filters};
use actix_web::{web, HttpResponse};
use serde_qs::actix::QsQuery;
use sqlx;
use sqlx::PgPool;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct RecordQuery {
    pub record_id: String,
}

#[tracing::instrument(name = "Getting records", skip(query, pool))]
pub async fn query_records(
    query: Option<QsQuery<Filters>>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, GetFilterError> {
    let records = advanced_record_filtering(query.as_ref(), &pool)
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
