use crate::record::RecordAdd;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

pub async fn add(record: web::Json<RecordAdd>, pool: web::Data<PgPool>) -> HttpResponse {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new record to database.",
        %request_id,
        record_id = %record.record_id,
    );
    let _request_span_guard = request_span.enter();
    let query_span = tracing::info_span!("Adding new record to the database");
    let runtime = match record.stop_time.as_ref() {
        Some(&stop) => Some((stop - record.start_time).num_seconds()),
        _ => None,
    };

    match sqlx::query_unchecked!(
        r#"
        INSERT INTO accounting (
            record_id, site_id, user_id, group_id,
            components, start_time, stop_time, runtime, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        record.record_id,
        record.site_id,
        record.user_id,
        record.group_id,
        record.components,
        record.start_time,
        record.stop_time,
        runtime,
        Utc::now()
    )
    .execute(pool.as_ref())
    .instrument(query_span)
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
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
