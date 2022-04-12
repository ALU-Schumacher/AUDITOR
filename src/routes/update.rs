use crate::record::RecordUpdate;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

pub async fn update(record: web::Json<RecordUpdate>, pool: web::Data<PgPool>) -> HttpResponse {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Updating a record.",
        %request_id,
        record_id = %record.record_id,
    );
    let _request_span_guard = request_span.enter();
    let query_span = tracing::info_span!("Getting record to be updated from database.");
    let r = match sqlx::query!(
        r#"
        SELECT start_time
        FROM accounting 
        WHERE record_id = $1
        "#,
        record.record_id,
    )
    .fetch_one(pool.get_ref())
    .instrument(query_span)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(
                "request_id {} - Failed to execute query: {:?}",
                request_id,
                e
            );
            return HttpResponse::BadRequest().finish();
        }
    };

    let query_span = tracing::info_span!("Updating record in database.");
    match sqlx::query_unchecked!(
        r#"
        UPDATE accounting
        SET stop_time = $6,
            runtime = $7,
            updated_at = $8
        WHERE
            record_id = $1 and site_id = $2 and user_id = $3 and group_id = $4 and components = $5
        "#,
        record.record_id,
        record.site_id,
        record.user_id,
        record.group_id,
        record.components,
        record.stop_time,
        (record.stop_time - r.start_time).num_seconds(),
        Utc::now()
    )
    .execute(pool.get_ref())
    .instrument(query_span)
    .await
    {
        Ok(_) => {
            tracing::info!(
                "request_id {} - Record {} updated",
                request_id,
                record.record_id
            );
            HttpResponse::Ok().finish()
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
