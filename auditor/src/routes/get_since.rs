// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::{Component, Record, RecordDatabase};
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use sqlx;
use sqlx::PgPool;
use std::fmt;

#[derive(thiserror::Error)]
pub enum GetSinceError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

debug_for_error!(GetSinceError);
responseerror_for_error!(GetSinceError, UnexpectedError => INTERNAL_SERVER_ERROR;);

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum StartedStopped {
    Started,
    Stopped,
}

impl fmt::Display for StartedStopped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[tracing::instrument(
    name = "Getting records since a timestamp",
    skip(info, pool),
    fields(
        startedstopped = %info.0,
        date = %info.1,
    )
)]
pub async fn get_since(
    info: web::Path<(StartedStopped, DateTime<Utc>)>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, GetSinceError> {
    let records = get_records_since(&info, &pool)
        .await
        .map_err(GetSinceError::UnexpectedError)?;
    Ok(HttpResponse::Ok().json(records))
}

#[tracing::instrument(name = "Get all records since a given timepoint", skip(info, pool))]
pub async fn get_records_since(
    info: &(StartedStopped, DateTime<Utc>),
    pool: &PgPool,
) -> Result<Vec<Record>, anyhow::Error> {
    match info.0 {
        StartedStopped::Started => {
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
                   WHERE a.start_time > $1 and a.runtime IS NOT NULL
                   ORDER BY a.stop_time
               "#,
               info.1,
            )
            .fetch_all(pool)
            .await
        }
        StartedStopped::Stopped => {
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
                WHERE a.stop_time > $1 and a.runtime IS NOT NULL
                ORDER BY a.stop_time
                "#,
                info.1,
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(GetRecordSinceError)?
    .into_iter()
    .map(Record::try_from)
    .collect::<Result<Vec<Record>, anyhow::Error>>()
}

pub struct GetRecordSinceError(sqlx::Error);

error_for_error!(GetRecordSinceError);
debug_for_error!(GetRecordSinceError);
display_for_error!(
    GetRecordSinceError,
    "A database error was encountered while trying to get a record from the database"
);
