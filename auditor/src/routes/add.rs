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

    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => return Err(AddRecordError(e)),
    };

    let id = sqlx::query_unchecked!(
        r#"
        INSERT INTO accounting (
            record_id, start_time, stop_time, runtime, updated_at
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id;
        "#,
        record.record_id.as_ref(),
        record.start_time,
        record.stop_time,
        runtime,
        Utc::now()
    )
    .fetch_optional(&mut *transaction)
    .await
    .map_err(AddRecordError)?
    .ok_or_else(|| AddRecordError(sqlx::Error::RowNotFound))?
    .id;

    for component in record.components.iter() {
        let (names, scores): (Vec<String>, Vec<f64>) = component
            .scores
            .iter()
            .map(|s| (s.name.as_ref().to_string(), s.value.as_ref()))
            .unzip();

        sqlx::query_unchecked!(
            r#"
            WITH insert_components AS (
                INSERT INTO components (name, amount)
                VALUES ($1, $2)
                RETURNING id
            ),
            insert_scores AS (
                INSERT INTO scores (name, value)
                SELECT * FROM UNNEST($3::text[], $4::double precision[])
                -- Update if already in table. This isn't great, but 
                -- otherwise RETURNING won't return anything.
                ON CONFLICT (name, value) DO UPDATE
                SET value = EXCLUDED.value, name = EXCLUDED.name
                RETURNING id
            ),
            insert_components_scores AS (
                INSERT INTO components_scores (component_id, score_id)
                SELECT (SELECT id FROM insert_components), id
                FROM insert_scores
            )
            INSERT INTO records_components (record_id, component_id)
            SELECT $5, (SELECT id from insert_components) 
            "#,
            component.name.as_ref(),
            component.amount,
            &names[..],
            &scores[..],
            id,
        )
        .execute(&mut *transaction)
        .await
        .map_err(AddRecordError)?;
    }

    if let Some(data) = record.meta.as_ref() {
        let data = data.to_vec();

        let (rid, names, values): (Vec<String>, Vec<String>, Vec<String>) =
            itertools::multiunzip(data.into_iter().flat_map(|(k, v)| {
                v.into_iter()
                    .map(|v| (record.record_id.as_ref().to_string(), k.clone(), v))
                    .collect::<Vec<_>>()
            }));

        sqlx::query!(
            r#"
            INSERT INTO meta (record_id, key, value)
            SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[])
            "#,
            &rid[..],
            &names[..],
            &values[..],
        )
        .execute(&mut *transaction)
        .await
        .map_err(AddRecordError)?;
    }

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
) -> Result<HttpResponse, AddError> {
    bulk_insert(&records, &pool)
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
    let updated_at_vec: Vec<_> = std::iter::repeat(Utc::now()).take(records.len()).collect();

    let ids = sqlx::query_unchecked!(
        r#"
        INSERT INTO accounting (
            record_id, start_time, stop_time, runtime, updated_at
        )
        SELECT * FROM UNNEST($1::text[], $2::timestamptz[], $3::timestamptz[], $4::bigint[], $5::timestamptz[])
        RETURNING id;
        "#,
        &record_ids[..],
        &start_times[..],
        &stop_times[..],
        &runtimes[..],
        &updated_at_vec[..],
    )
    .fetch_all(&mut *transaction)
    .await
    .map_err(AddRecordError)?;

    for (record, id) in records.iter().zip(ids.iter()) {
        for component in record.components.iter() {
            let (names, scores): (Vec<String>, Vec<f64>) = component
                .scores
                .iter()
                .map(|s| (s.name.as_ref().to_string(), s.value.as_ref()))
                .unzip();

            sqlx::query_unchecked!(
                r#"
                WITH insert_components AS (
                    INSERT INTO components (name, amount)
                    VALUES ($1, $2)
                    RETURNING id
                ),
                insert_scores AS (
                    INSERT INTO scores (name, value)
                    SELECT * FROM UNNEST($3::text[], $4::double precision[])
                    -- Update if already in table. This isn't great, but 
                    -- otherwise RETURNING won't return anything.
                    ON CONFLICT (name, value) DO UPDATE
                    SET value = EXCLUDED.value, name = EXCLUDED.name
                    RETURNING id
                ),
                insert_components_scores AS (
                    INSERT INTO components_scores (component_id, score_id)
                    SELECT (SELECT id FROM insert_components), id
                    FROM insert_scores
                )
                INSERT INTO records_components (record_id, component_id)
                SELECT $5, (SELECT id from insert_components) 
                "#,
                component.name.as_ref(),
                component.amount,
                &names[..],
                &scores[..],
                &id.id,
            )
            .execute(&mut *transaction)
            .await
            .map_err(AddRecordError)?;
        }

        if let Some(data) = record.meta.as_ref() {
            let data = data.to_vec();

            let (rid, names, values): (Vec<String>, Vec<String>, Vec<String>) =
                itertools::multiunzip(data.into_iter().flat_map(|(k, v)| {
                    v.into_iter()
                        .map(|v| (record.record_id.as_ref().to_string(), k.clone(), v))
                        .collect::<Vec<_>>()
                }));

            sqlx::query!(
                r#"
                INSERT INTO meta (record_id, key, value)
                SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[])
                "#,
                &rid[..],
                &names[..],
                &values[..],
            )
            .execute(&mut *transaction)
            .await
            .map_err(AddRecordError)?;
        }
    }

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
