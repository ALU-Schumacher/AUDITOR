// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::{Component, Record, RecordDatabase, UnitMeta};
use actix_web::{web, HttpResponse};
use sqlx;
use sqlx::PgPool;

#[derive(thiserror::Error)]
pub enum GetError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

debug_for_error!(GetError);
responseerror_for_error!(GetError, UnexpectedError => INTERNAL_SERVER_ERROR;);

#[tracing::instrument(name = "Getting all records from database", skip(pool))]
pub async fn get(pool: web::Data<PgPool>) -> Result<HttpResponse, GetError> {
    let records = get_records(&pool)
        .await
        .map_err(GetError::UnexpectedError)?;
    Ok(HttpResponse::Ok().json(records))
}

#[tracing::instrument(name = "Retrieving records from database", skip(pool))]
pub async fn get_records(pool: &PgPool) -> Result<Vec<Record>, anyhow::Error> {
    sqlx::query_as!(
        RecordDatabase,
        r#"SELECT a.record_id,
                  m.meta as "meta: Vec<UnitMeta>",
                  a.site_id,
                  a.user_id,
                  a.group_id,
                  a.components as "components: Vec<Component>",
                  a.start_time as "start_time?",
                  a.stop_time,
                  a.runtime
           FROM accounting a
           LEFT JOIN (
               SELECT m.record_id as record_id, array_agg(m.meta) as meta 
               FROM meta as m
               GROUP BY m.record_id
               ) m ON m.record_id = a.record_id
           ORDER BY a.stop_time;
            "#,
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
