use crate::record::RecordAdd;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx;
use sqlx::PgPool;

pub async fn add(record: web::Json<RecordAdd>, pool: web::Data<PgPool>) -> HttpResponse {
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
    .execute(pool.get_ref())
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            println!("Failed to execute query: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
