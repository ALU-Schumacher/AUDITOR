// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::{Record, RecordDatabase};
use sqlx::PgPool;

#[derive(thiserror::Error)]
pub enum GetError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

debug_for_error!(GetError);
responseerror_for_error!(GetError, UnexpectedError => INTERNAL_SERVER_ERROR;);

#[tracing::instrument(name = "Retrieving records from database", skip(pool))]
pub async fn get_records(pool: &PgPool) -> Result<Vec<Record>, anyhow::Error> {
    sqlx::query_as!(
        RecordDatabase,
        r#"SELECT record_id,
                  meta,
                  components,
                  start_time,
                  stop_time,
                  runtime
           FROM auditor_accounting
           ORDER BY stop_time
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(GetRecordError)?
    .into_iter()
    .map(Record::try_from)
    .collect::<Result<Vec<Record>, anyhow::Error>>()
}

pub struct GetRecordError(sqlx::Error);

error_for_error!(GetRecordError);
debug_for_error!(GetRecordError);
display_for_error!(
    GetRecordError,
    "A database error was encountered while trying to get a record from the database."
);
