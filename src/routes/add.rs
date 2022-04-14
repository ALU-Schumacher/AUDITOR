use crate::record::RecordAdd;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx;
use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(
    name = "Adding a record to the database",
    skip(record, pool),
    fields(
        request_id = %Uuid::new_v4(),
        record_id = %record.record_id,
    )
)]
pub async fn add(record: web::Json<RecordAdd>, pool: web::Data<PgPool>) -> HttpResponse {
    match add_record(&record, &pool).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Inserting record into database", skip(record, pool))]
pub async fn add_record(record: &RecordAdd, pool: &PgPool) -> Result<(), sqlx::Error> {
    let runtime = match record.stop_time.as_ref() {
        Some(&stop) => Some((stop - record.start_time).num_seconds()),
        _ => None,
    };

    sqlx::query_unchecked!(
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
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
