use crate::record::{Component, Record};
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use sqlx;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StartedStopped {
    Started,
    Stopped,
}

pub async fn get_since(
    info: web::Path<(StartedStopped, DateTime<Utc>)>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let request_id = Uuid::new_v4();
    let (startstop, date) = info.into_inner();
    match startstop {
        StartedStopped::Started => {
            let request_span = tracing::info_span!(
                "Getting all records started since a given date.",
                %request_id,
                %date,
            );
            let _request_span_guard = request_span.enter();
            let query_span = tracing::info_span!("Retrieving records from database");
            match sqlx::query_as!(
                Record,
                r#"SELECT
                   record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
                   start_time, stop_time, runtime
                   FROM accounting
                   WHERE start_time > $1 and runtime IS NOT NULL
                "#,
                date
            )
            .fetch_all(&**pool)
            .instrument(query_span)
            .await
            {
                Ok(records) => {
                    tracing::info!(
                        "request_id {} - Received all records started since {}",
                        request_id,
                        date
                    );
                    HttpResponse::Ok().json(records)
                }
                Err(e) => {
                    tracing::error!(
                        "request_id {} - Failed to execute query: {:?}",
                        request_id,
                        e
                    );
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        StartedStopped::Stopped => {
            let request_span = tracing::info_span!(
                "Getting all records stopped after a given date.",
                %request_id,
                %date,
            );
            let _request_span_guard = request_span.enter();
            let query_span = tracing::info_span!("Retrieving records from database");
            match sqlx::query_as!(
                Record,
                r#"SELECT
                   record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
                   start_time, stop_time, runtime
                   FROM accounting
                   WHERE stop_time > $1 and runtime IS NOT NULL
                "#,
                date
            )
            .fetch_all(&**pool)
            .instrument(query_span)
            .await
            {
                Ok(records) => {
                    tracing::info!(
                        "request_id {} - Received all records stopped since {}",
                        request_id,
                        date
                    );
                    HttpResponse::Ok().json(records)
                }
                Err(e) => {
                    tracing::error!(
                        "request_id {} - Failed to execute query: {:?}",
                        request_id,
                        e
                    );
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
    }
}
