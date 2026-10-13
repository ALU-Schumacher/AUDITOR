use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;

#[derive(Debug, thiserror::Error)]
pub enum OpenStackError {
    #[error("A record with this record_id already exists")]
    RecordExists,
    #[error("Record not found")]
    NotFound,
    #[error("Record is already finalized (stop_time set)")]
    AlreadyFinished,
    #[error("{0}")]
    BadRequest(&'static str),
    #[error("Unexpected error")]
    Unexpected(#[from] sqlx::Error),
}

impl ResponseError for OpenStackError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::RecordExists | Self::AlreadyFinished => StatusCode::CONFLICT,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    fn error_response(&self) -> HttpResponse {
        let msg = match self {
            Self::Unexpected(_) => "Unexpected error".to_string(),
            other => other.to_string(),
        };
        HttpResponse::build(self.status_code()).body(msg)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct ComponentUpdate {
    /// Name of an existing component, e.g. "TotalCPU"
    pub name: String,
    pub amount: i64,
}

#[derive(serde::Deserialize, Debug)]
pub struct OpenStackUpdate {
    pub record_id: String,
    pub components: Vec<ComponentUpdate>,
    pub stop_time: Option<DateTime<Utc>>,
}

#[tracing::instrument(name = "Updating an OpenStack record", skip(pool))]
pub async fn openstack_update(
    body: web::Json<OpenStackUpdate>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, OpenStackError> {
    let upd = body.into_inner();
    let record_id = upd.record_id.clone();

    if upd.components.is_empty() {
        return Err(OpenStackError::BadRequest("components must not be empty"));
    }
    let incoming: Value = serde_json::to_value(&upd.components)
        .map_err(|_| OpenStackError::BadRequest("invalid components"))?;

    let result = sqlx::query_unchecked!(
        r#"
        UPDATE auditor_accounting
        SET
            components = COALESCE((
                SELECT jsonb_agg(
                    COALESCE(
                        (SELECT jsonb_set(t.c, '{amount}', u->'amount')
                         FROM jsonb_array_elements($2::jsonb) AS u
                         WHERE u->>'name' = t.c->>'name'
                         LIMIT 1),
                        t.c
                    )
                    ORDER BY t.ord
                )
                FROM jsonb_array_elements(components) WITH ORDINALITY AS t(c, ord)
            ), '[]'::jsonb),
            stop_time  = COALESCE($3::timestamptz, stop_time),
            runtime    = CASE WHEN $3::timestamptz IS NOT NULL
                              THEN EXTRACT(EPOCH FROM ($3::timestamptz - start_time))::bigint
                              ELSE runtime END,
            updated_at = $4
        WHERE record_id = $1
          AND stop_time IS NULL
          AND ($3::timestamptz IS NULL OR $3::timestamptz >= start_time)
          AND NOT EXISTS (
              SELECT 1 FROM jsonb_array_elements($2::jsonb) AS u
              WHERE NOT EXISTS (
                  SELECT 1 FROM jsonb_array_elements(components) AS c
                  WHERE c->>'name' = u->>'name'
              )
          )
        "#,
        record_id,
        incoming,
        upd.stop_time,
        Utc::now()
    )
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() == 1 {
        return Ok(HttpResponse::Ok().finish());
    }

    let row = sqlx::query_unchecked!(
        r#"
        SELECT
            stop_time AS "stop_time?",
            NOT EXISTS (
                SELECT 1 FROM jsonb_array_elements($2::jsonb) AS u
                WHERE NOT EXISTS (
                    SELECT 1 FROM jsonb_array_elements(components) AS c
                    WHERE c->>'name' = u->>'name'
                )
            ) AS "names_ok!"
        FROM auditor_accounting
        WHERE record_id = $1
        "#,
        record_id,
        incoming
    )
    .fetch_optional(pool.get_ref())
    .await?;

    match row {
        None => Err(OpenStackError::NotFound),
        Some(r) if r.stop_time.is_some() => Err(OpenStackError::AlreadyFinished),
        Some(r) if !r.names_ok => Err(OpenStackError::BadRequest(
            "one or more component names do not exist on this record",
        )),
        Some(_) => Err(OpenStackError::BadRequest("stop_time is before start_time")),
    }
}
