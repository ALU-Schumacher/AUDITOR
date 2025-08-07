// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::{Record, RecordDatabase, ValidAmount, ValidName};
use chrono::{DateTime, Utc};
use core::fmt::Debug;
use sqlx::{PgPool, QueryBuilder, Row};
use std::collections::HashMap;
use std::fmt::Display;

use actix_web::web::Bytes;
use async_stream::stream;
use futures::Stream;
use futures_util::TryStreamExt;

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Filters {
    pub record_id: Option<ValidName>,
    pub start_time: Option<Operator<DateTime<Utc>>>,
    pub stop_time: Option<Operator<DateTime<Utc>>>,
    pub runtime: Option<Operator<ValidAmount>>,
    pub meta: Option<HashMap<ValidName, MetaOperator>>,
    pub component: Option<HashMap<ValidName, Operator<ValidAmount>>>,
    pub sort_by: Option<SortOption>,
    pub limit: Option<ValidAmount>,
    //#[serde(skip)]
    //pub allowed_site_ids: Option<Vec<String>>,
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

    pub fn enforce_meta_filtering(
        &mut self,
        mut allowed_meta_ids: HashMap<String, Vec<String>>,
    ) -> Result<(), String> {
        if allowed_meta_ids.is_empty() {
            return Err("No meta IDs available for the user".to_string());
        }

        if let Some(meta_filters) = &mut self.meta {
            for (key, meta_operator) in meta_filters.clone() {
                if let Some(requested_meta_ids) = &meta_operator.c {
                    let all_authorized = requested_meta_ids.to_vec().iter().all(|site| {
                        allowed_meta_ids
                            .get(&key.to_string())
                            .expect("meta_id does not exist")
                            .contains(&site.to_string())
                    });

                    if !all_authorized {
                        return Err(format!(
                            "Permission denied to access the site with the meta_id {}",
                            &key
                        ));
                    }
                    return Ok(());
                }
                if let Some(dnc_sites) = &meta_operator.dnc {
                    if let Some(sites) = allowed_meta_ids.get_mut(&key.to_string()) {
                        sites.retain(|site| {
                            !dnc_sites.contains(&ValidName::parse(site.to_string()).unwrap())
                        });
                    }

                    if allowed_meta_ids
                        .get(&key.to_string())
                        .expect("meta_id does not exist")
                        .is_empty()
                    {
                        return Err(
                            "Permission denied to access the site with the meta_id".to_string()
                        );
                    }
                    return Ok(());
                }
            }

            if !allowed_meta_ids.is_empty() {
                for (key, values) in allowed_meta_ids.iter() {
                    let valid_values: Vec<ValidName> = values
                        .iter()
                        .map(|v| ValidName::parse(v.clone()).unwrap())
                        .collect();

                    meta_filters.insert(
                        ValidName::parse(key.clone()).unwrap(),
                        MetaOperator {
                            c: Some(valid_values),
                            dnc: None,
                        },
                    );
                }
            }
        } else if !allowed_meta_ids.is_empty() {
            let mut meta_filters = HashMap::new();

            for (key, values) in allowed_meta_ids.iter() {
                let valid_values: Vec<ValidName> = values
                    .iter()
                    .map(|v| ValidName::parse(v.clone()).unwrap())
                    .collect();

                meta_filters.insert(
                    ValidName::parse(key.clone()).unwrap(),
                    MetaOperator {
                        c: Some(valid_values),
                        dnc: None,
                    },
                );
            }

            self.meta = Some(meta_filters);
        }

        Ok(())
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Operator<T> {
    pub gt: Option<T>,
    pub lt: Option<T>,
    pub gte: Option<T>,
    pub lte: Option<T>,
    pub equals: Option<T>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct MetaOperator {
    pub c: Option<Vec<ValidName>>,
    pub dnc: Option<Vec<ValidName>>,
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

impl Display for SortField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortField::StartTime => write!(f, "start_time"),
            SortField::StopTime => write!(f, "stop_time"),
            SortField::Runtime => write!(f, "runtime"),
            SortField::RecordId => write!(f, "record_id"),
        }
    }
}

#[tracing::instrument(name = "Getting records using custom query", skip(filters, pool))]
pub async fn advanced_record_filtering(
    filters: Filters,
    pool: PgPool,
) -> impl Stream<Item = Result<Bytes, anyhow::Error>> {
    let mut query = QueryBuilder::new(
        "SELECT record_id,
                  meta,
                  components,
                  start_time,
                  stop_time,
                  runtime
           FROM auditor_accounting
               ",
    );

    if filters.start_time.is_some()
        || filters.stop_time.is_some()
        || filters.runtime.is_some()
        || filters.meta.is_some()
        || filters.component.is_some()
        || filters.record_id.is_some()
    {
        query.push(" WHERE ".to_string());
        if let Some(record_id) = &filters.record_id {
            // query string -> a.record_id = '{}' and
            query.push(" record_id = ".to_string());
            query.push_bind(record_id.clone());
            query.push(" and ".to_string());
        }

        if let Some(start_time_filters) = &filters.start_time
            && let Some(operators) = get_operator(start_time_filters)
        {
            for operator in operators {
                // query string -> a.start_time {} '{}' and
                query.push(format!(" start_time {} ", operator.0));
                query.push_bind(*operator.1);
                query.push(" and ".to_string());
            }
        }

        if let Some(stop_time_filters) = &filters.stop_time
            && let Some(operators) = get_operator(stop_time_filters)
        {
            for operator in operators {
                // query string -> a.stop_time {} '{}' and
                query.push(format!(" stop_time {} ", operator.0));
                query.push_bind(*operator.1);
                query.push(" and ".to_string());
            }
        }

        if let Some(meta_filters) = &filters.meta {
            for (key, meta_operator) in meta_filters {
                if let Some(c) = &meta_operator.c {
                    // query string -> meta -> "site_id" @> jsonb_build_array("site1") and
                    c.to_vec().iter().for_each(|site| {
                        query.push(" meta ->  ".to_string());
                        query.push_bind(key.clone());
                        query.push(" @> jsonb_build_array(".to_string());
                        query.push_bind(site.clone());
                        query.push(") ");
                        query.push(" and ");
                    });
                }
                if let Some(dnc) = &meta_operator.dnc {
                    // query string -> NOT (meta -> "site_id" @> jsonb_build_array("site_1")) and

                    dnc.to_vec().iter().for_each(|site| {
                        query.push(" NOT (meta ->  ".to_string());
                        query.push_bind(key.clone());
                        query.push(" @> jsonb_build_array(".to_string());
                        query.push_bind(site.clone());
                        query.push(") ) ");
                        query.push(" and ");
                    });
                }
            }
        }

        if let Some(component_filters) = &filters.component {
            for (key, component_operator) in component_filters {
                if let Some(operators) = get_operator(component_operator) {
                    for operator in operators {
                        // query string -> components->0->>'name' = "CPU" AND
                        // (components->0->>'amount')::int >10  and

                        query.push("components->0->>'name' = ");
                        query.push_bind(key.clone());
                        query.push(format!(
                            " AND (components->0->>'amount')::int {} ",
                            &operator.0
                        ));
                        query.push_bind(*operator.1);

                        query.push(" and ".to_string());
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
                    // query string ->  a.runtime {} {} and
                    query.push(format!(" runtime {} ", operator.0));
                    query.push_bind(*operator.1);
                    query.push(" and ".to_string());
                }
            }
        } else {
            query.push(" runtime IS NOT NULL".to_string());
        }
    }

    if let Some(sort_by) = &filters.sort_by {
        if let SortOption::ASC(asc) = sort_by {
            query.push(format!(" ORDER BY {} ASC", &asc.to_string()));
        }
        if let SortOption::DESC(desc) = sort_by {
            query.push(format!(" ORDER BY {} DESC", &desc.to_string()));
        }
    } else {
        query.push(" ORDER BY stop_time ".to_string());
    }

    if let Some(limit) = &filters.limit {
        query.push(" LIMIT ".to_string());
        query.push_bind(*limit);
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
        if let Some(equals) = &operator.equals
            && !is_datetime::<T>()
        {
            operators.push(("=", equals));
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

    stream! {

    let mut rows = query.build().persistent(false).fetch(&pool);

    while let Some(row) = rows.try_next().await.unwrap_or(None)

        {
            let beam_object = Record {
                record_id: row.try_get("record_id").unwrap(),
                meta: row.try_get("meta").ok().and_then(|value| serde_json::from_value(value).ok()),
                components: row.try_get("components").ok().and_then(|value| serde_json::from_value(value).ok()),
                start_time: row.try_get("start_time").ok(),
                stop_time: row.try_get("stop_time").ok(),
                runtime: row.try_get("runtime").ok(),
            };

            let json_bytes = serde_json::to_vec(&beam_object).unwrap().into();
            yield Ok(json_bytes);
        }
    }
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
            r#"SELECT record_id,
                  meta,
                  components,
                  start_time,
                  stop_time,
                  runtime
           FROM auditor_accounting
           WHERE record_id = $1
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
