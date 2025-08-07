// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::constants::{ERR_RECORD_EXISTS, ERR_UNEXPECTED_ERROR};
use crate::domain::RecordAdd;
use actix_web::{HttpResponse, ResponseError, web};
use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;

#[derive(thiserror::Error)]
pub enum AddError {
    RecordExists,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    // UnexpectedError,
}

#[derive(serde::Serialize)]
struct WarningResponse {
    warning: String,
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
    ignore_record_exists_error: web::Data<bool>,
) -> Result<HttpResponse, AddError> {
    match add_record(&record, &pool).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => match e.0.as_database_error() {
            Some(db_err) => match db_err.code() {
                Some(code) if code == "23505" => {
                    if **ignore_record_exists_error {
                        tracing::warn!(
                            "!! ----- RECORD ALREADY EXISTS – IGNORING DUE TO CONFIGURATION. ----- !!"
                        );
                        let body = WarningResponse {
        warning: "Record already exists, but the error was ignored due to AUDITOR configuration.".into(),
    };
                        Ok(HttpResponse::Ok().json(body))
                    } else {
                        Err(AddError::RecordExists)
                    }
                }
                _ => Err(AddError::UnexpectedError(e.into())),
            },
            None => Err(AddError::UnexpectedError(e.into())),
        },
    }
}

#[tracing::instrument(name = "Inserting record into database", skip(record, pool))]
pub async fn add_record(record: &RecordAdd, pool: &PgPool) -> Result<(), AddRecordError> {
    let runtime = match record.stop_time.as_ref() {
        Some(&stop) => Some((stop - record.start_time).num_seconds()),
        _ => None,
    };

    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => return Err(AddRecordError(e)),
    };

    sqlx::query_unchecked!(
        r#"
        INSERT INTO auditor_accounting (
            record_id, start_time, stop_time, meta, components, runtime, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id;
        "#,
        record.record_id.as_ref(),
        record.start_time,
        record.stop_time,
        serde_json::to_value(&record.meta).unwrap_or_else(|_| serde_json::Value::Null),
        serde_json::to_value(&record.components).unwrap_or_else(|_| serde_json::Value::Null),
        runtime,
        Utc::now()
    )
    .fetch_optional(&mut *transaction)
    .await
    .map_err(AddRecordError)?
    .ok_or_else(|| AddRecordError(sqlx::Error::RowNotFound))?;

    if let Err(e) = transaction.commit().await {
        Err(AddRecordError(e))
    } else {
        Ok(())
    }
}

#[tracing::instrument(name = "Adding multiple records to the database", skip(records, pool))]
pub async fn bulk_add(
    records: web::Json<Vec<RecordAdd>>,
    pool: web::Data<PgPool>,
    ignore_record_exists_error: web::Data<bool>,
) -> Result<HttpResponse, AddError> {
    match bulk_insert(&records, &pool).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),

        Err(e) => {
            if let Some(db_err) = e.0.as_database_error()
                && let Some(code) = db_err.code().as_ref()
                && code == "23505"
            {
                if **ignore_record_exists_error {
                    tracing::warn!(
                        "!! ----- ONE OR MORE RECORDS ALREADY EXIST – IGNORING DUE TO CONFIGURATION. ----- !!"
                    );

                    let body = WarningResponse {
        warning: "One or more records already exist, but the error was ignored due to AUDITOR configuration.".into(),
    };
                    return Ok(HttpResponse::Ok().json(body));
                } else {
                    return Err(AddError::RecordExists);
                }
            }
            Err(AddError::UnexpectedError(e.into()))
        }
    }
}

#[tracing::instrument(name = "Inserting bulk records into database", skip(records, pool))]
pub async fn bulk_insert(records: &[RecordAdd], pool: &PgPool) -> Result<(), AddRecordError> {
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => return Err(AddRecordError(e)),
    };

    let record_ids: Vec<_> = records
        .iter()
        .map(|r| r.record_id.as_ref().to_string())
        .collect();
    let start_times: Vec<_> = records.iter().map(|r| r.start_time).collect();
    let stop_times: Vec<_> = records.iter().map(|r| r.stop_time).collect();
    let runtimes: Vec<_> = records
        .iter()
        .map(|r| r.stop_time.map(|stop| (stop - r.start_time).num_seconds()))
        .collect();
    let updated_at_vec: Vec<_> = std::iter::repeat_n(Utc::now(), records.len()).collect();

    let meta_values: Vec<Value> = records
        .iter()
        .map(|r| serde_json::to_value(&r.meta).unwrap_or(serde_json::Value::Null))
        .collect();
    let component_values: Vec<Value> = records
        .iter()
        .map(|r| serde_json::to_value(&r.components).unwrap_or(serde_json::Value::Null))
        .collect();

    sqlx::query_unchecked!(
        r#"
        INSERT INTO auditor_accounting (
            record_id, start_time, stop_time, meta, components, runtime, updated_at
        )
        SELECT * FROM UNNEST($1::text[], $2::timestamptz[], $3::timestamptz[], $4::jsonb[], $5::jsonb[],  $6::bigint[], $7::timestamptz[])
        RETURNING id;
        "#,
        &record_ids[..],
        &start_times[..],
        &stop_times[..],
        &meta_values[..],
        &component_values[..],
        &runtimes[..],
        &updated_at_vec[..],
    )
    .fetch_all(&mut *transaction)
    .await
    .map_err(AddRecordError)?;

    if let Err(e) = transaction.commit().await {
        return Err(AddRecordError(e));
    } else {
        return Ok(());
    }
}

pub struct AddRecordError(sqlx::Error);

debug_for_error!(AddRecordError);
error_for_error!(AddRecordError);
display_for_error!(
    AddRecordError,
    "A database error was encountered while trying to store a record."
);
