use crate::routes::{advanced_record_filtering, Filters};
use crate::routes::{GetFilterError, StartedStopped};
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde_qs::actix::QsQuery;
use sqlx;
use sqlx::PgPool;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct RecordQuery {
    pub state: StartedStopped,
    pub since: DateTime<Utc>,
}

#[tracing::instrument(name = "Getting Records", skip(query, pool))]
pub async fn query_records(
    query: Option<QsQuery<Filters>>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, GetFilterError> {
    let records = advanced_record_filtering(query.as_ref(), &pool)
        .await
        .map_err(GetFilterError::UnexpectedError)?;
    Ok(HttpResponse::Ok().json(records))
}
