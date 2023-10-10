// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::{Component, Record, RecordDatabase};
use sqlx;
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
        r#"SELECT a.record_id,
                  m.meta as "meta: Vec<(String, Vec<String>)>",
                  css.components as "components: Vec<Component>",
                  a.start_time as "start_time?",
                  a.stop_time,
                  a.runtime
           FROM accounting a
           LEFT JOIN (
               WITH subquery AS (
                   SELECT m.record_id as record_id, m.key as key, array_agg(m.value) as values
                   FROM meta as m
                   GROUP BY m.record_id, m.key
               )
               SELECT s.record_id as record_id, array_agg(row(s.key, s.values)) as meta
               FROM subquery as s
               GROUP BY s.record_id
               ) m ON m.record_id = a.record_id
           LEFT JOIN (
               WITH subquery AS (
                  SELECT 
                      c.id as cid,
                      COALESCE(array_agg(row(s.name, s.value)::score) FILTER (WHERE s.name IS NOT NULL AND s.value IS NOT NULL), '{}'::score[]) as scores
                  FROM components as c
                  LEFT JOIN components_scores as cs
                  ON c.id = cs.component_id
                  LEFT JOIN scores as s
                  ON cs.score_id = s.id
                  GROUP BY c.id
               )
               SELECT rc.record_id as id, array_agg(row(c.name, c.amount, sq.scores)::component) as components
               FROM records_components AS rc
               LEFT JOIN components as c
               ON rc.component_id = c.id
               LEFT JOIN subquery AS sq
               ON sq.cid = rc.component_id
               GROUP BY rc.record_id
           ) css ON css.id = a.id
           ORDER BY a.stop_time
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
