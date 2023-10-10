use crate::routes::{get_records, get_records_since, GetSinceError, StartedStopped};
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use sqlx;
use sqlx::PgPool;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct RecordQuery {
    pub state: StartedStopped,
    pub since: DateTime<Utc>,
}

#[tracing::instrument(name = "Getting records", skip(record_query, pool))]
pub async fn query_records(
    record_query: Option<web::Query<RecordQuery>>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, GetSinceError> {
    match record_query {
        Some(query) => {
            // Handle "get since" with query parameters
            let info = (query.state.clone(), query.since);
            let records = get_records_since(&info, &pool)
                .await
                .map_err(GetSinceError::UnexpectedError)?;
            Ok(HttpResponse::Ok().json(records))
        }

        _ => {
            // Handle "get all records" (no query parameters)
            let records = get_records(&pool)
                .await
                .map_err(GetSinceError::UnexpectedError)?;
            Ok(HttpResponse::Ok().json(records))
        }
    }
}
