// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::configuration::Settings;
use prometheus::core::{Collector, Desc};
use prometheus::proto::MetricFamily;
use prometheus::{IntGauge, IntGaugeVec, Opts};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

struct AggregatedColumns {
    name: String,
    num: i64,
}

impl AggregatedColumns {
    fn into_tuple(self) -> (String, i64) {
        (self.name, self.num)
    }
}

#[derive(Clone)]
pub struct DatabaseMetricsWatcher {
    db_pool: PgPool,
    data: Arc<Mutex<DatabaseMetricsData>>,
    desc: Desc,
    frequency: chrono::Duration,
    metrics: Vec<DatabaseMetricsOptions>,
    meta_key_site: String,
    meta_key_group: String,
    meta_key_user: String,
}

struct DatabaseMetricsData {
    num_records: Option<i64>,
    num_records_per_site: Option<HashMap<String, i64>>,
    num_records_per_group: Option<HashMap<String, i64>>,
    num_records_per_user: Option<HashMap<String, i64>>,
}

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseMetricsOptions {
    RecordCount,
    RecordCountPerSite,
    RecordCountPerGroup,
    RecordCountPerUser,
}

impl DatabaseMetricsWatcher {
    pub fn new(pool: PgPool, config: &Settings) -> Result<DatabaseMetricsWatcher, anyhow::Error> {
        let desc = Desc::new(
            "database_metrics".to_string(),
            "Metrics from the Auditor database".to_string(),
            vec![],
            std::collections::HashMap::new(),
        )?;

        Ok(DatabaseMetricsWatcher {
            db_pool: pool,
            data: Arc::new(Mutex::new(DatabaseMetricsData {
                num_records: None,
                num_records_per_site: None,
                num_records_per_group: None,
                num_records_per_user: None,
            })),
            desc,
            frequency: config.metrics.database.frequency,
            metrics: config.metrics.database.metrics.clone(),
            meta_key_site: config.metrics.database.meta_key_site.clone(),
            meta_key_group: config.metrics.database.meta_key_group.clone(),
            meta_key_user: config.metrics.database.meta_key_user.clone(),
        })
    }

    #[tracing::instrument(name = "Monitoring database for metrics", skip(self))]
    pub async fn monitor(&self) -> Result<(), anyhow::Error> {
        let mut interval = tokio::time::interval(self.frequency.to_std()?);
        loop {
            interval.tick().await;
            for metric in self.metrics.iter() {
                match metric {
                    DatabaseMetricsOptions::RecordCount => self.update_record_count().await?,
                    DatabaseMetricsOptions::RecordCountPerSite => {
                        self.update_record_count_per_site().await?
                    }
                    DatabaseMetricsOptions::RecordCountPerGroup => {
                        self.update_record_count_per_group().await?
                    }
                    DatabaseMetricsOptions::RecordCountPerUser => {
                        self.update_record_count_per_user().await?
                    }
                };
            }
        }
    }

    #[tracing::instrument(name = "Update record count for database metrics", skip(self))]
    async fn update_record_count(&self) -> Result<(), anyhow::Error> {
        let num = sqlx::query_scalar!(r#"SELECT count(*) as "count!" FROM auditor_accounting;"#)
            .fetch_one(&self.db_pool)
            .await?;
        let mut data_lock = self.data.lock().unwrap();
        data_lock.num_records = Some(num);
        Ok(())
    }

    #[tracing::instrument(name = "Update record count per site for database metrics", skip(self))]
    async fn update_record_count_per_site(&self) -> Result<(), anyhow::Error> {
        let per_site: HashMap<String, i64> = sqlx::query_as!(
            AggregatedColumns,
            r#"
            SELECT jsonb_array_elements_text(meta->$1) AS "name!", COUNT(*) AS "num!"
            FROM auditor_accounting
            GROUP BY jsonb_array_elements_text(meta->$1);
            "#,
            self.meta_key_site
        )
        .fetch_all(&self.db_pool)
        .await?
        .into_iter()
        .map(AggregatedColumns::into_tuple)
        .collect();

        let mut data_lock = self.data.lock().unwrap();
        data_lock.num_records_per_site = Some(per_site);
        Ok(())
    }

    #[tracing::instrument(
        name = "Update record count per group for database metrics",
        skip(self)
    )]
    async fn update_record_count_per_group(&self) -> Result<(), anyhow::Error> {
        let per_group: HashMap<String, i64> = sqlx::query_as!(
            AggregatedColumns,
            r#"
            SELECT jsonb_array_elements_text(meta->$1) AS "name!", COUNT(*) AS "num!"
            FROM auditor_accounting
            GROUP BY jsonb_array_elements_text(meta->$1);
            "#,
            self.meta_key_group
        )
        .fetch_all(&self.db_pool)
        .await?
        .into_iter()
        .map(AggregatedColumns::into_tuple)
        .collect();

        let mut data_lock = self.data.lock().unwrap();
        data_lock.num_records_per_group = Some(per_group);
        Ok(())
    }

    #[tracing::instrument(name = "Update record count per user for database metrics", skip(self))]
    async fn update_record_count_per_user(&self) -> Result<(), anyhow::Error> {
        let per_user: HashMap<String, i64> = sqlx::query_as!(
            AggregatedColumns,
            r#"
            SELECT jsonb_array_elements_text(meta->$1) AS "name!", COUNT(*) AS "num!"
            FROM auditor_accounting
            GROUP BY jsonb_array_elements_text(meta->$1);
            "#,
            self.meta_key_user
        )
        .fetch_all(&self.db_pool)
        .await?
        .into_iter()
        .map(AggregatedColumns::into_tuple)
        .collect();

        let mut data_lock = self.data.lock().unwrap();
        data_lock.num_records_per_user = Some(per_user);
        Ok(())
    }

    #[tracing::instrument(
        name = "Turning database metrics into gauges",
        skip(self)
        level = "debug"
    )]
    fn get_metrics(&self) -> Result<Vec<MetricFamily>, anyhow::Error> {
        let mut out = vec![];

        let data_lock = self.data.lock().unwrap();

        if let Some(num_records) = data_lock.num_records {
            let gauge = IntGauge::new(
                "num_records_database",
                "Number of records in the Auditor database",
            )?;
            gauge.set(num_records);
            out.extend(gauge.collect());
        }

        if let Some(ref num_records_per_site) = data_lock.num_records_per_site {
            let gauge_vec = IntGaugeVec::new(
                Opts::new(
                    "num_records_database_per_site",
                    "Number of records in the Auditor database",
                ),
                &["site"],
            )?;

            num_records_per_site
                .iter()
                .map(|(name, &num)| gauge_vec.with_label_values(&[&name[..]]).set(num))
                .count();

            out.extend(gauge_vec.collect());
        }

        if let Some(ref num_records_per_group) = data_lock.num_records_per_group {
            let gauge_vec = IntGaugeVec::new(
                Opts::new(
                    "num_records_database_per_group",
                    "Number of records in the Auditor database",
                ),
                &["group_id"],
            )?;

            num_records_per_group
                .iter()
                .map(|(name, &num)| gauge_vec.with_label_values(&[&name[..]]).set(num))
                .count();

            out.extend(gauge_vec.collect());
        }

        if let Some(ref num_records_per_user) = data_lock.num_records_per_user {
            let gauge_vec = IntGaugeVec::new(
                Opts::new(
                    "num_records_database_per_user",
                    "Number of records in the Auditor database",
                ),
                &["user_id"],
            )?;

            num_records_per_user
                .iter()
                .map(|(name, &num)| gauge_vec.with_label_values(&[&name[..]]).set(num))
                .count();

            out.extend(gauge_vec.collect());
        }

        Ok(out)
    }
}

impl Collector for DatabaseMetricsWatcher {
    fn desc(&self) -> Vec<&Desc> {
        vec![&self.desc]
    }

    #[tracing::instrument(name = "Prometheus collecting database metrics", skip(self))]
    fn collect(&self) -> Vec<MetricFamily> {
        self.get_metrics().unwrap()
    }
}
