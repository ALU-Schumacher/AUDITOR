use std::collections::HashMap;

use crate::routes::{Filters, advanced_record_filtering, get_one_record};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, ResponseError, web};
use futures_util::{StreamExt, TryStreamExt};
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
    enforce_rbac: web::Data<bool>,
) -> Result<HttpResponse, GetFilterError> {
    let query_string = query.query_string();

    let mut filters: Filters = match serde_qs::from_str(query_string) {
        Ok(filters) => filters,
        Err(err) => return Err(GetFilterError::InvalidQuery(err.to_string())),
    };

    if **enforce_rbac
        && let Some(meta_info) = query.extensions().get::<HashMap<String, Vec<String>>>()
    {
        let meta_info = meta_info.clone();

        match filters.enforce_meta_filtering(meta_info) {
            Ok(_) => {}
            Err(msg) => return Err(GetFilterError::UnexpectedError(msg)),
        }
    }

    let mut stream = Box::pin(advanced_record_filtering(filters, pool.as_ref().clone()).await);
    // Do not commit a successful response before the database query has succeeded.
    let first = stream
        .try_next()
        .await
        .map_err(|error| GetFilterError::UnexpectedError(error.to_string()))?;
    let stream = futures_util::stream::iter(first.map(Ok::<_, anyhow::Error>)).chain(stream);
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .streaming(stream))
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
    InvalidQuery(String),

    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
}

impl ResponseError for GetFilterError {
    fn error_response(&self) -> HttpResponse {
        match self {
            GetFilterError::InvalidQuery(msg) => {
                HttpResponse::BadRequest().json(json!({ "error": msg }))
            }
            GetFilterError::UnexpectedError(err) => {
                HttpResponse::InternalServerError().json(json!({ "error": err }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, http::StatusCode, test};
    use sqlx::postgres::PgPoolOptions;

    #[actix_web::test]
    async fn database_failure_returns_error_instead_of_empty_success() {
        // A closed lazy pool fails without contacting a database server.
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://test:test@127.0.0.1/test")
            .unwrap();
        pool.close().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .app_data(web::Data::new(false))
                .route("/records", web::get().to(query_records)),
        )
        .await;
        for uri in ["/records", "/records?limit=10"] {
            let response =
                test::call_service(&app, test::TestRequest::get().uri(uri).to_request()).await;
            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}
