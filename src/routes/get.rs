use crate::record::{Component, Record};
use actix_web::{web, HttpResponse};
use sqlx;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

pub async fn get(pool: web::Data<PgPool>) -> HttpResponse {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Getting all records from database",
        %request_id,
    );
    let _request_span_guard = request_span.enter();
    let query_span = tracing::info_span!("Retrieving records from database");
    tracing::info!(
        "request_id {} - Getting all records from database",
        request_id
    );
    match sqlx::query_as!(
        Record,
        r#"SELECT
           record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
           start_time, stop_time, runtime
           FROM accounting
        "#,
    )
    .fetch_all(&**pool)
    .instrument(query_span)
    .await
    {
        Ok(records) => {
            tracing::info!("request_id {} - Returned records.", request_id);
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
