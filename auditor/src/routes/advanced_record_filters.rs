// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::{Component, Record, RecordDatabase, ValidName};
use chrono::{DateTime, Utc};
use core::fmt::Debug;
use sqlx;
use sqlx::{PgPool, QueryBuilder, Row};
use std::collections::HashMap;

#[derive(thiserror::Error)]
pub enum GetFilterError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

debug_for_error!(GetFilterError);
responseerror_for_error!(GetFilterError, UnexpectedError => INTERNAL_SERVER_ERROR;);

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Filters {
    pub record_id: Option<String>,
    pub start_time: Option<Operator<DateTime<Utc>>>,
    pub stop_time: Option<Operator<DateTime<Utc>>>,
    pub runtime: Option<Operator<String>>,
    pub meta: Option<HashMap<String, MetaOperator>>,
    pub component: Option<HashMap<String, Operator<u8>>>,
    pub sort_by: Option<SortOption>,
    pub limit: Option<u64>,
}

impl Filters {
    pub fn is_all_none(&self) -> bool {
        self.record_id.is_none()
            && self.start_time.is_none()
            && self.stop_time.is_none()
            && self.runtime.is_none()
            && self.meta.is_none()
            && self.component.is_none()
            && self.sort_by.is_none()
            && self.limit.is_none()
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Operator<T> {
    pub gt: Option<T>,
    pub lt: Option<T>,
    pub gte: Option<T>,
    pub lte: Option<T>,
    pub equals: Option<T>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct MetaOperator {
    pub c: Option<String>,
    pub dnc: Option<String>,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SortOption {
    ASC(SortField),
    DESC(SortField),
}

#[derive(serde::Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SortField {
    #[serde(rename = "start_time")]
    StartTime,
    #[serde(rename = "stop_time")]
    StopTime,
    #[serde(rename = "runtime")]
    Runtime,
    #[serde(rename = "record_id")]
    RecordId,
}

impl ToString for SortField {
    fn to_string(&self) -> String {
        match self {
            SortField::StartTime => String::from("start_time"),
            SortField::StopTime => String::from("stop_time"),
            SortField::Runtime => String::from("runtime"),
            SortField::RecordId => String::from("record_id"),
        }
    }
}

#[tracing::instrument(name = "Getting records using custom query", skip(filters, pool))]
pub async fn advanced_record_filtering(
    filters: Filters,
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

    if filters.start_time.is_some()
        || filters.stop_time.is_some()
        || filters.runtime.is_some()
        || filters.record_id.is_some()
    {
        query.push(" WHERE ".to_string());
        if let Some(record_id) = &filters.record_id {
            let is_valid_record_id = ValidName::parse(record_id.clone().to_string());
            if is_valid_record_id.is_ok() {
                query.push(format!("a.record_id = '{}' and ", &record_id));
            }
        }

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
        }

        if let Some(meta_filters) = &filters.meta {
            for (key, meta_operator) in meta_filters {
                if let Some(c) = &meta_operator.c {
                    let meta_value = ValidName::parse(c.clone().to_string());
                    if meta_value.is_ok() {
                        query.push(format!("Array['{}'] = ANY(SELECT r.values FROM unnest(m.meta) AS r(key text, values text[]) WHERE r.key = '{}') and ", &c, &key));
                    } else {
                        println!("meta value validation Failed")
                    }
                }
                if let Some(dnc) = &meta_operator.dnc {
                    let meta_value = ValidName::parse(dnc.clone().to_string());
                    if meta_value.is_ok() {
                        query.push(format!(" (NOT EXISTS (SELECT r.values FROM unnest(css.components) AS r(key text, values text[]) WHERE r.key = '{}' AND Array['{}'] @> r.values)) and ", &key, &dnc));
                    } else {
                        println!("meta value verification failed");
                    }
                }
            }
        }

        if let Some(component_filters) = &filters.component {
            for (key, component_operator) in component_filters {
                if let Some(operators) = get_operator(component_operator) {
                    for operator in operators {
                        query.push(format!(
                            "css.components[1].name = '{}' AND css.components[2].amount {} {} and ",
                            &key, &operator.0, &operator.1
                        ));
                    }
                }
            }
        }

        // The previous implementation of get and get_since is replicated. Getting all records also includes
        // the records whose runtime IS NOT NULL. But while querying with the start_time or stop_time,
        // we also specify the query to only include the records whose runtime is NOT NULL

        if let Some(runtime_filters) = &filters.runtime {
            if let Some(operators) = get_operator(runtime_filters) {
                for operator in operators {
                    query.push(format!("a.runtime {} {} and ", operator.0, operator.1));
                }
            }
        } else {
            query.push("a.runtime IS NOT NULL".to_string());
        }
    }

    if let Some(sort_by) = &filters.sort_by {
        if let SortOption::ASC(asc) = sort_by {
            query.push(format!("ORDER BY a.{} ASC", &asc.to_string()));
        }
        if let SortOption::DESC(desc) = sort_by {
            query.push(format!("ORDER BY a.{} DESC", &desc.to_string()));
        }
    } else {
        query.push(" ORDER BY a.stop_time ".to_string());
    }

    if let Some(limit) = &filters.limit {
        query.push(format!(" LIMIT {}", &limit));
    }

    fn get_operator<T>(operator: &Operator<T>) -> Option<Vec<(&str, &T)>>
    where
        T: 'static,
    {
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
        if let Some(equals) = &operator.equals {
            if !is_datetime::<T>() {
                operators.push(("=", equals));
            }
        }
        if !operators.is_empty() {
            Some(operators)
        } else {
            None
        }
    }

    // Helper function to check if T is Datetime
    fn is_datetime<T: 'static>() -> bool {
        std::any::TypeId::of::<T>() == std::any::TypeId::of::<DateTime<Utc>>()
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

#[tracing::instrument(name = "Getting one record using record_id", skip(record_id, pool))]
pub async fn get_one_record(
    record_id: String,
    pool: &PgPool,
) -> Result<Option<Record>, anyhow::Error> {
    let is_valid_record_id = ValidName::parse(record_id.clone().to_string());
    if is_valid_record_id.is_ok() {
        Ok(sqlx::query_as!(
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
                   WHERE a.record_id = $1
               "#,
               &record_id,
            )
            .fetch_one(pool)
            .await
            .map(Record::try_from)
            .map_err(GetRecordError)?
            .ok())
    } else {
        return Ok(None);
    }
}

struct GetRecordError(sqlx::Error);
error_for_error!(GetRecordError);
debug_for_error!(GetRecordError);
display_for_error!(
    GetRecordError,
    "A database error was encountered while trying to get a record from the database"
);
