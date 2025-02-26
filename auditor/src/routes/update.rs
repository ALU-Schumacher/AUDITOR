// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::RecordUpdate;
use actix_web::{HttpResponse, web};
use chrono::Utc;
use sqlx::PgPool;

#[derive(thiserror::Error)]
pub enum UpdateError {
    #[error("Updating unknown record {0} not possible.")]
    UnknownRecord(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

debug_for_error!(UpdateError);
responseerror_for_error!(
    UpdateError,
    UnknownRecord => NOT_FOUND;
    UnexpectedError => INTERNAL_SERVER_ERROR;
);

#[tracing::instrument(
    name = "Updating a record",
    skip(record, pool),
    fields(record_id = %record.record_id)
)]
pub async fn update(
    record: web::Json<RecordUpdate>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, UpdateError> {
    update_record(&record, &pool).await.map_err(|e| match e {
        UpdateRecordError::RowNotFoundError(s) => UpdateError::UnknownRecord(s),
        UpdateRecordError::OtherError(err) => UpdateError::UnexpectedError(err.into()),
    })?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Updating a record in the database", skip(record, pool))]
pub async fn update_record(record: &RecordUpdate, pool: &PgPool) -> Result<(), UpdateRecordError> {
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => return Err(UpdateRecordError::OtherError(e)),
    };

    let start_time = sqlx::query!(
        r#"
        SELECT start_time
        FROM auditor_accounting
        WHERE record_id = $1
        "#,
        record.record_id.as_ref(),
    )
    .fetch_one(&mut *transaction)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => {
            UpdateRecordError::RowNotFoundError(record.record_id.as_ref().into())
        }
        e => UpdateRecordError::OtherError(e),
    })?
    .start_time;

    sqlx::query_unchecked!(
        r#"
        UPDATE auditor_accounting
        SET stop_time = $2,
            runtime = $3,
            updated_at = $4
        WHERE
            record_id = $1
        "#,
        record.record_id.as_ref(),
        record.stop_time,
        (record.stop_time - start_time).num_seconds(),
        Utc::now()
    )
    .execute(&mut *transaction)
    .await
    .map_err(UpdateRecordError::OtherError)?;

    if let Err(e) = transaction.commit().await {
        Err(UpdateRecordError::OtherError(e))
    } else {
        Ok(())
    }
}

#[derive(thiserror::Error)]
pub enum UpdateRecordError {
    #[error("Entry {0} not found in database")]
    RowNotFoundError(String),
    #[error(transparent)]
    OtherError(#[from] sqlx::Error),
}

debug_for_error!(UpdateRecordError);
