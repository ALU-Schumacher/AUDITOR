use crate::domain::{Component, Record};
use actix_web::{web, HttpResponse};
use sqlx;
use sqlx::PgPool;

#[tracing::instrument(name = "Getting all records from database", skip(pool))]
pub async fn get(pool: web::Data<PgPool>) -> HttpResponse {
    match get_records(&pool).await {
        Ok(records) => HttpResponse::Ok().json(records),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Retrieving records from database", skip(pool))]
pub async fn get_records(pool: &PgPool) -> Result<Vec<Record>, sqlx::Error> {
    sqlx::query_as!(
        Record,
        r#"SELECT
           record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
           start_time, stop_time, runtime
           FROM accounting
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })
}
