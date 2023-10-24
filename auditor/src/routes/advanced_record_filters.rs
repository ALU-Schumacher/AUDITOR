// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::{Record, RecordDatabase};
use chrono::{DateTime, Utc};
use core::fmt::Debug;
use serde_qs::actix::QsQuery;
use sqlx;
use sqlx::{PgPool, QueryBuilder, Row};

#[derive(thiserror::Error)]
pub enum GetFilterError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

debug_for_error!(GetFilterError);
responseerror_for_error!(GetFilterError, UnexpectedError => INTERNAL_SERVER_ERROR;);

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Filters {
    pub start_time: Option<Operator<DateTime<Utc>>>,
    pub stop_time: Option<Operator<DateTime<Utc>>>,
    pub runtime: Option<Operator<String>>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Operator<T> {
    pub gt: Option<T>,
    pub lt: Option<T>,
    pub gte: Option<T>,
    pub lte: Option<T>,
}

#[tracing::instrument(name = "Get all records since a given timepoint", skip(filters, pool))]
pub async fn advanced_record_filtering(
    filters: Option<&QsQuery<Filters>>,
    pool: &PgPool,
) -> Result<Vec<Record>, anyhow::Error> {
    let mut query = QueryBuilder::new("SELECT a.record_id,
                          m.meta,
                          css.components,
                          a.start_time,
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
               ");

    if let Some(filters) = &filters {
        if filters.start_time.is_some() || filters.stop_time.is_some() || filters.runtime.is_some()
        {
            query.push(" WHERE ".to_string());
            if let Some(start_time_filters) = &filters.start_time {
                if let Some(operators) = get_operator(start_time_filters) {
                    for operator in operators {
                        let formatted_datetime = operator.1.format("%Y-%m-%d %H:%M:%S").to_string();
                        query.push(format!(
                            "a.start_time {} '{}' and ",
                            operator.0, &formatted_datetime
                        ));
                    }
                }
            } else {
                println!("start_time is not specified to query");
            }

            if let Some(stop_time_filters) = &filters.stop_time {
                if let Some(operators) = get_operator(stop_time_filters) {
                    for operator in operators {
                        let formatted_datetime = operator.1.format("%Y-%m-%d %H:%M:%S").to_string();
                        query.push(format!(
                            "a.stop_time {} '{}' and ",
                            operator.0, &formatted_datetime
                        ));
                    }
                }
            } else {
                println!("stop_time is not specified to query");
            }

            if let Some(runtime_filters) = &filters.runtime {
                if let Some(operators) = get_operator(runtime_filters) {
                    for operator in operators {
                        query.push(format!("a.runtime {} {} and ", operator.0, operator.1));
                    }
                }
            } else {
                println!("runtime is not specified to query");
                query.push("a.runtime IS NOT NULL".to_string());
            }
        }
    } else {
        println!("Fetching all records")
    }

    // The previous implementation of get and get_since is replicated. Getting all records also includes
    // the records whose runtime IS NOT NULL. But while querying with the start_time or stop_time,
    // we also specify the query to only include the records whose runtime is NOT NULL
    query.push(" ORDER BY a.stop_time".to_string());

    fn get_operator<T>(operator: &Operator<T>) -> Option<Vec<(&str, &T)>> {
        let mut operators: Vec<(&str, &T)> = Vec::new();

        if operator.gt.is_some() && operator.gte.is_some()
            || operator.lt.is_some() && operator.lte.is_some()
        {
            return None;
        }

        if let Some(gt) = &operator.gt {
            operators.push((">", gt));
        }
        if let Some(lt) = &operator.lt {
            operators.push(("<", lt));
        }
        if let Some(gte) = &operator.gte {
            operators.push((">=", gte));
        }
        if let Some(lte) = &operator.lte {
            operators.push(("<=", lte));
        }
        if !operators.is_empty() {
            Some(operators)
        } else {
            None
        }
    }

    let rows = query
        .build()
        .fetch_all(pool)
        .await
        .map_err(GetRecordError)?;

    let result: Vec<RecordDatabase> = rows
        .iter()
        .map(|row| RecordDatabase {
            record_id: row.try_get("record_id").unwrap(),
            meta: row.try_get("meta").ok().unwrap_or(None),
            components: row.try_get("components").ok().unwrap_or(None),
            start_time: row.try_get("start_time").ok().unwrap_or(None),
            stop_time: row.try_get("stop_time").ok().unwrap_or(None),
            runtime: row.try_get("runtime").ok().unwrap_or(None),
        })
        .collect();

    result
        .into_iter()
        .map(Record::try_from)
        .collect::<Result<Vec<Record>, anyhow::Error>>()
}

struct GetRecordError(sqlx::Error);
error_for_error!(GetRecordError);
debug_for_error!(GetRecordError);
display_for_error!(
    GetRecordError,
    "A database error was encountered while trying to get a record from the database"
);
