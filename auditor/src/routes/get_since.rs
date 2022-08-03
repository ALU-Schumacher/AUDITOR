// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::{Component, Record};
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use sqlx;
use sqlx::PgPool;
use std::fmt;

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum StartedStopped {
    Started,
    Stopped,
}

impl fmt::Display for StartedStopped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
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
) -> HttpResponse {
    match get_records_since(&info, &pool).await {
        Ok(records) => HttpResponse::Ok().json(records),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Get all records since a given timepoint", skip(info, pool))]
pub async fn get_records_since(
    info: &(StartedStopped, DateTime<Utc>),
    pool: &PgPool,
) -> Result<Vec<Record>, sqlx::Error> {
    match info.0 {
        StartedStopped::Started => {
            sqlx::query_as!(
                Record,
                r#"SELECT
                record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
                start_time, stop_time, runtime
                FROM accounting
                WHERE start_time > $1 and runtime IS NOT NULL
                "#,
                info.1,
            )
            .fetch_all(pool)
            .await
        }
        StartedStopped::Stopped => {
            sqlx::query_as!(
                Record,
                r#"SELECT
                record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
                start_time, stop_time, runtime
                FROM accounting
                WHERE stop_time > $1 and runtime IS NOT NULL
                "#,
                info.1,
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })
}
