use crate::record::RecordUpdate;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx;
use sqlx::PgPool;

pub async fn update(record: web::Json<RecordUpdate>, pool: web::Data<PgPool>) -> HttpResponse {
    let r = match sqlx::query!(
        r#"
        SELECT start_time
        FROM accounting 
        WHERE record_id = $1
        "#,
        record.record_id,
    )
    .fetch_one(pool.get_ref())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("Failed to execute query: {}", e);
            return HttpResponse::BadRequest().finish();
        }
    };

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
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            println!("Failed to execute query: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
