use crate::domain::RecordUpdate;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx;
use sqlx::PgPool;

#[tracing::instrument(
    name = "Updating a record",
    skip(record, pool),
    fields(record_id = %record.record_id)
)]
pub async fn update(record: web::Json<RecordUpdate>, pool: web::Data<PgPool>) -> HttpResponse {
    match update_record(&record, &pool).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => match e {
            // TODO: See if this can be solved better
            sqlx::Error::RowNotFound => HttpResponse::BadRequest().finish(),
            _ => HttpResponse::InternalServerError().finish(),
        },
    }
}

#[tracing::instrument(name = "Updating a record in the database", skip(record, pool))]
pub async fn update_record(record: &RecordUpdate, pool: &PgPool) -> Result<(), sqlx::Error> {
    // TODO: Can and probably should be merged into a single query.
    let start_time = sqlx::query!(
        r#"
        SELECT start_time
        FROM accounting 
        WHERE record_id = $1
        "#,
        record.record_id.as_ref(),
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?
    .start_time;

    sqlx::query_unchecked!(
        r#"
        UPDATE accounting
        SET stop_time = $6,
            runtime = $7,
            updated_at = $8
        WHERE
            record_id = $1 and site_id = $2 and user_id = $3 and group_id = $4 and components = $5
        "#,
        record.record_id.as_ref(),
        record.site_id.as_ref(),
        record.user_id.as_ref(),
        record.group_id.as_ref(),
        record.components,
        record.stop_time,
        (record.stop_time - start_time).num_seconds(),
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
