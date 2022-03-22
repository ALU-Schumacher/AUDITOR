use crate::record::{Component, Record};
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use sqlx;
use sqlx::PgPool;

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
    let (startstop, date) = info.into_inner();
    match startstop {
        StartedStopped::Started => match sqlx::query_as!(
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
        .await
        {
            Ok(records) => HttpResponse::Ok().json(records),
            Err(e) => {
                println!("Failed to execute query: {}", e);
                HttpResponse::InternalServerError().finish()
            }
        },
        StartedStopped::Stopped => match sqlx::query_as!(
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
        .await
        {
            Ok(records) => HttpResponse::Ok().json(records),
            Err(e) => {
                println!("Failed to execute query: {}", e);
                HttpResponse::InternalServerError().finish()
            }
        },
    }
}
