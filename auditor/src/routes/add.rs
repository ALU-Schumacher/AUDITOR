// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::constants::{ERR_RECORD_EXISTS, ERR_UNEXPECTED_ERROR};
use crate::domain::RecordAdd;
use actix_web::{web, HttpResponse, ResponseError};
use chrono::Utc;
use sqlx;
use sqlx::PgPool;

#[derive(thiserror::Error)]
pub enum AddError {
    RecordExists,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    // UnexpectedError,
}

debug_for_error!(AddError);
// responseerror_for_error!(AddError, UnexpectedError => INTERNAL_SERVER_ERROR;);

impl std::fmt::Display for AddError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AddError::RecordExists => ERR_RECORD_EXISTS,
                AddError::UnexpectedError(_) => ERR_UNEXPECTED_ERROR,
            }
        )
    }
}

impl ResponseError for AddError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            AddError::UnexpectedError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            AddError::RecordExists => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let message = match self {
            AddError::UnexpectedError(_) => ERR_UNEXPECTED_ERROR,
            AddError::RecordExists => ERR_RECORD_EXISTS,
        };

        HttpResponse::build(self.status_code()).body(message)
    }
}

#[tracing::instrument(
    name = "Adding a record to the database",
    skip(record, pool),
    fields(record_id = %record.record_id)
)]
pub async fn add(
    record: web::Json<RecordAdd>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, AddError> {
    add_record(&record, &pool)
        .await
        .map_err(|e| match e.0.as_database_error() {
            Some(db_err) => match db_err.code().as_ref() {
                Some(code) => match code.as_ref() {
                    "23505" => AddError::RecordExists,
                    _ => AddError::UnexpectedError(e.into()),
                },
                _ => AddError::UnexpectedError(e.into()),
            },
            _ => AddError::UnexpectedError(e.into()),
        })?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Inserting record into database", skip(record, pool))]
pub async fn add_record(record: &RecordAdd, pool: &PgPool) -> Result<(), AddRecordError> {
    let runtime = match record.stop_time.as_ref() {
        Some(&stop) => Some((stop - record.start_time).num_seconds()),
        _ => None,
    };

    sqlx::query_unchecked!(
        r#"
        INSERT INTO accounting (
            record_id, meta, site_id, user_id, group_id,
            components, start_time, stop_time, runtime, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
        record.record_id.as_ref(),
        record.meta.to_vec_unit(),
        record.site_id.as_ref(),
        record.user_id.as_ref(),
        record.group_id.as_ref(),
        record.components,
        record.start_time,
        record.stop_time,
        runtime,
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(AddRecordError)?;

    Ok(())
}

pub struct AddRecordError(sqlx::Error);

debug_for_error!(AddRecordError);
error_for_error!(AddRecordError);
display_for_error!(
    AddRecordError,
    "A database error was encountered while trying to store a record."
);
