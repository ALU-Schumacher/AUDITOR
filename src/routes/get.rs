use crate::record::{Component, Record};
use actix_web::{web, HttpResponse};
use sqlx;
use sqlx::PgPool;

pub async fn get(pool: web::Data<PgPool>) -> HttpResponse {
    match sqlx::query_as!(
        Record,
        r#"SELECT
           record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
           start_time, stop_time, runtime
           FROM accounting
        "#,
    )
    .fetch_all(&**pool)
    .await
    {
        Ok(records) => HttpResponse::Ok().json(records),
        Err(e) => {
            println!("Failed to execute query: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
