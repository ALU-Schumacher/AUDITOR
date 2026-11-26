// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::configuration::Settings;
use chrono::{DateTime, DurationRound, Utc};
use prometheus::core::{Collector, Desc};
use prometheus::proto::{Gauge, LabelPair, Metric, MetricFamily, MetricType};
use prometheus::{IntGauge, IntGaugeVec, Opts};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

/// Prometheus rejects samples older than ~1h behind its newest data unless
/// `out_of_order_time_window` is set, and we don't want to require that.
/// Therefore we add 55 min to make sure it lies within the prometheus's default
/// out of order window.
const SAMPLE_OFFSET_MINUTES: i64 = 55;

/// Stop exposing a snapshot once its sample is older than this, so Prometheus is
/// not fed samples it would reject (and warn about) on every scrape.
const MAX_EXPOSE_AGE_MINUTES: i64 = 50;

/// Deprecated legacy metrics. They are only computed when `enable_wlcg_metrics`
/// is `false`; when it is `true` all four are skipped.
#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseMetricsOptions {
    RecordCount,
    RecordCountPerSite,
    RecordCountPerGroup,
    RecordCountPerUser,
}

/// Row of the legacy "records per site/group/user" queries.
#[derive(sqlx::FromRow)]
struct AggregatedColumns {
    name: String,
    num: i64,
}

impl AggregatedColumns {
    fn into_tuple(self) -> (String, i64) {
        (self.name, self.num)
    }
}

/// One aggregated row of the WLCG dashboard: all records that stopped in one
/// hour for one label set.
#[derive(sqlx::FromRow, Debug, Clone)]
struct HourlyRow {
    site: String,
    user: String,
    submithost: String,
    group: String,
    infrastructure: String,
    core_hours: f64,
    score_hours: f64,
    cpu_hours: f64,
    cpu_score_hours: f64,
    records: i64,
}

/// One row of the generic dashboard query: summed component amount for one
/// (meta key, meta value, component) combination.
#[derive(sqlx::FromRow, Debug, Clone)]
struct GenericRow {
    meta_key: String,
    meta_value: String,
    component: String,
    amount: f64,
}

/// Aggregation of one complete hour: records with `stop_time` in `[start, start + 1h)`.
/// Samples are stamped with `start + SAMPLE_OFFSET_MINUTES`.
struct HourlySnapshot<T> {
    hour: DateTime<Utc>, // sample timestamp
    rows: Vec<T>,
}

struct DatabaseMetricsData {
    num_records: Option<i64>,
    num_records_per_site: Option<HashMap<String, i64>>,
    num_records_per_group: Option<HashMap<String, i64>>,
    num_records_per_user: Option<HashMap<String, i64>>,
    /// WLCG dashboard snapshot.
    hourly: Option<HourlySnapshot<HourlyRow>>,
    /// Generic dashboard snapshot.
    generic: Option<HourlySnapshot<GenericRow>>,
}

#[derive(Clone)]
pub struct DatabaseMetricsWatcher {
    db_pool: PgPool,
    data: Arc<Mutex<DatabaseMetricsData>>,
    desc: Desc,
    /// Deprecated legacy metrics (only active if `enable_wlcg_metrics` is false).
    metrics: Vec<DatabaseMetricsOptions>,
    /// Refresh interval of the legacy record count metrics.
    frequency: StdDuration,
    /// Generic hourly dashboard.
    enable_metrics: bool,
    excluded_meta_keys: Vec<String>,
    excluded_components: Vec<String>,
    /// WLCG hourly dashboard.
    enable_wlcg_metrics: bool,
    /// Grace period after an hour ends, shared by both dashboards.
    wlcg_metrics_delay: chrono::Duration,
    benchmark_name: String,
    component_name_cores: String,
    component_name_total_cpu: String,
    meta_key_site: Option<String>,
    meta_key_user: Option<String>,
    meta_key_submithost: Option<String>,
    meta_key_group: Option<String>,
    meta_key_infrastructure: Option<String>,
}

const HOURLY_SQL: &str = r#"
SELECT
    COALESCE(NULLIF(a.meta->$1::text->>0, ''), 'unknown') AS site,
    COALESCE(NULLIF(a.meta->$2::text->>0, ''), 'unknown') AS user,
    COALESCE(NULLIF(a.meta->$3::text->>0, ''), 'unknown') AS submithost,
    COALESCE(NULLIF(a.meta->$4::text->>0, ''), 'unknown') AS group,
    COALESCE(NULLIF(a.meta->$5::text->>0, ''), 'unknown') AS infrastructure,
    COALESCE(SUM(c.cores * a.runtime) / 3600.0, 0)::float8 AS core_hours,
    COALESCE(SUM(c.cores * c.score * a.runtime) / 3600.0, 0)::float8 AS score_hours,
    COALESCE(SUM(c.total_cpu) / 3600.0, 0)::float8 AS cpu_hours,
    COALESCE(SUM(c.total_cpu * c.score) / 3600.0, 0)::float8 AS cpu_score_hours,
    COUNT(*)::int8 AS records
FROM auditor_accounting a
CROSS JOIN LATERAL (
    SELECT
        MAX((component->>'amount')::numeric)
            FILTER (WHERE lower(component->>'name') = lower($9::text)) AS cores,
        MAX((component->>'amount')::numeric)
            FILTER (WHERE lower(component->>'name') = lower($10::text)) AS total_cpu,
        MAX((score->>'value')::numeric)
            FILTER (WHERE lower(score->>'name') = lower($8::text)) AS score
    FROM jsonb_array_elements(a.components) AS component
    LEFT JOIN LATERAL jsonb_array_elements(
        CASE
            WHEN lower(component->>'name') = lower($9::text)
            THEN COALESCE(component->'scores', '[]'::jsonb)
            ELSE '[]'::jsonb
        END
    ) AS score ON TRUE
) c
WHERE a.stop_time >= $6 AND a.stop_time < $7
GROUP BY 1, 2, 3, 4, 5
"#;

/// Generic dashboard: summed component amount per meta key/value and component,
/// for records that stopped within one hour.
/// $1 = excluded meta keys, $2 = excluded component names,
/// $3 = hour start, $4 = hour end (records with stop_time in [start, end)).
const GENERIC_SQL: &str = r#"
SELECT
    m.key AS meta_key,
    v.value AS meta_value,
    c.component->>'name' AS component,
    COALESCE(SUM((c.component->>'amount')::double precision), 0)::float8 AS amount
FROM auditor_accounting a
CROSS JOIN LATERAL jsonb_each(a.meta) m
CROSS JOIN LATERAL jsonb_array_elements_text(
    CASE
        WHEN jsonb_typeof(m.value) = 'array' THEN m.value
        ELSE jsonb_build_array(m.value)
    END
) v(value)
CROSS JOIN LATERAL jsonb_array_elements(a.components) AS c(component)
WHERE a.stop_time >= $3 AND a.stop_time < $4
  AND v.value IS NOT NULL
  AND m.key <> ALL($1::text[])
  AND c.component->>'name' <> ALL($2::text[])
GROUP BY m.key, v.value, c.component->>'name'
"#;

/// Legacy query: number of records per value of a meta key (array valued).
/// Non-array values are ignored instead of making the query fail.
const COUNT_PER_KEY_SQL: &str = r#"
SELECT elem AS name, COUNT(*)::int8 AS num
FROM auditor_accounting,
     jsonb_array_elements_text(
         CASE
             WHEN jsonb_typeof(meta->$1::text) = 'array' THEN meta->$1::text
             ELSE '[]'::jsonb
         END
     ) AS elem
GROUP BY elem
"#;

/// A meta key that is `None` or empty means "this field does not exist":
/// the label is then always `unknown`.
fn non_empty(key: &Option<String>) -> Option<String> {
    key.clone().filter(|k| !k.trim().is_empty())
}

/// Builds a gauge family with one label from a `name -> count` map.
fn per_key_gauge(
    name: &str,
    label: &str,
    map: &HashMap<String, i64>,
) -> Result<Vec<MetricFamily>, anyhow::Error> {
    let gauge_vec = IntGaugeVec::new(
        Opts::new(name, "Number of records in the Auditor database"),
        &[label],
    )?;
    for (key, &num) in map {
        gauge_vec.with_label_values(&[key.as_str()]).set(num);
    }
    Ok(gauge_vec.collect())
}

fn label_pair(n: &str, v: &str) -> LabelPair {
    let mut l = LabelPair::default();
    l.set_name(n.to_string());
    l.set_value(v.to_string());
    l
}

/// Builds the timestamped generic dashboard family from a snapshot.
fn generic_family(snap: &HourlySnapshot<GenericRow>) -> MetricFamily {
    let ts_ms = snap.hour.timestamp_millis();
    let metrics: Vec<Metric> = snap
        .rows
        .iter()
        .map(|r| {
            let mut g = Gauge::default();
            g.set_value(r.amount);
            let mut m = Metric::default();
            m.set_label(
                vec![
                    label_pair("component", &r.component),
                    label_pair("meta_key", &r.meta_key),
                    label_pair("meta_value", &r.meta_value),
                ]
                .into(),
            );
            m.set_gauge(g);
            // Timestamp = start of hour + SAMPLE_OFFSET_MINUTES.
            m.set_timestamp_ms(ts_ms);
            m
        })
        .collect();

    let mut fam = MetricFamily::default();
    fam.set_name("auditor_component_amount".to_string());
    fam.set_help(
        "Summed component amount per meta key, meta value and component for records stopped in the hour"
            .to_string(),
    );
    fam.set_field_type(MetricType::GAUGE);
    fam.set_metric(metrics.into());
    fam
}

impl DatabaseMetricsWatcher {
    pub fn new(pool: PgPool, config: &Settings) -> Result<DatabaseMetricsWatcher, anyhow::Error> {
        let desc = Desc::new(
            "database_metrics".to_string(),
            "Metrics from the Auditor database".to_string(),
            vec![],
            std::collections::HashMap::new(),
        )?;
        let c = &config.metrics.database;

        if !c.metrics.is_empty() {
            tracing::warn!(
                "The database metrics RecordCount, RecordCountPerSite, RecordCountPerGroup \
                 and RecordCountPerUser are deprecated and have moved to `enable_wlcg_metrics`. \
                 Please set `enable_wlcg_metrics = true` instead."
            );
            if c.enable_wlcg_metrics {
                tracing::warn!(
                    "`enable_wlcg_metrics` is true: RecordCount, RecordCountPerSite, \
                     RecordCountPerGroup and RecordCountPerUser are skipped."
                );
            }
        }

        Ok(DatabaseMetricsWatcher {
            db_pool: pool,
            data: Arc::new(Mutex::new(DatabaseMetricsData {
                num_records: None,
                num_records_per_site: None,
                num_records_per_group: None,
                num_records_per_user: None,
                hourly: None,
                generic: None,
            })),
            desc,
            metrics: c.metrics.clone(),
            frequency: c.frequency.to_std()?,
            enable_metrics: c.enable_metrics,
            excluded_meta_keys: c.dashboard_excluded_meta_keys.clone(),
            excluded_components: c.dashboard_excluded_components.clone(),
            enable_wlcg_metrics: c.enable_wlcg_metrics,
            wlcg_metrics_delay: c.wlcg_metrics_delay,
            benchmark_name: c.benchmark_name.clone(),
            component_name_cores: c.component_name_cores.clone(),
            component_name_total_cpu: c.component_name_total_cpu.clone(),
            meta_key_site: non_empty(&c.meta_key_site),
            meta_key_user: non_empty(&c.meta_key_user),
            meta_key_submithost: non_empty(&c.meta_key_submithost),
            meta_key_group: non_empty(&c.meta_key_group),
            meta_key_infrastructure: non_empty(&c.meta_key_infrastructure),
        })
    }

    #[tracing::instrument(name = "Monitoring database for metrics", skip(self))]
    pub async fn monitor(&self) -> Result<(), anyhow::Error> {
        tokio::try_join!(self.run_legacy_metrics(), self.run_dashboard())?;
        Ok(())
    }

    /// Legacy metrics (RecordCount, RecordCountPerSite/Group/User).
    /// All four are skipped when the WLCG dashboard is enabled.
    async fn run_legacy_metrics(&self) -> Result<(), anyhow::Error> {
        if self.enable_wlcg_metrics || self.metrics.is_empty() {
            return Ok(());
        }
        let mut interval = tokio::time::interval(self.frequency);
        loop {
            interval.tick().await;
            for metric in self.metrics.iter() {
                let res = match metric {
                    DatabaseMetricsOptions::RecordCount => self.update_record_count().await,
                    DatabaseMetricsOptions::RecordCountPerSite => {
                        self.update_record_count_per_site().await
                    }
                    DatabaseMetricsOptions::RecordCountPerGroup => {
                        self.update_record_count_per_group().await
                    }
                    DatabaseMetricsOptions::RecordCountPerUser => {
                        self.update_record_count_per_user().await
                    }
                };
                if let Err(e) = res {
                    tracing::error!("Failed to update {metric:?}: {e:?}");
                }
            }
        }
    }

    /// Hourly scheduler shared by the generic and the WLCG dashboard.
    async fn run_dashboard(&self) -> Result<(), anyhow::Error> {
        if !self.enable_metrics && !self.enable_wlcg_metrics {
            return Ok(());
        }
        let one_hour = chrono::Duration::try_hours(1).expect("This should never fail");
        loop {
            // Latest hour that is complete, plus the grace period for late records.
            let hour_end = (Utc::now() - self.wlcg_metrics_delay).duration_trunc(one_hour)?;
            let hour_start = hour_end - one_hour;

            if self.enable_wlcg_metrics {
                if let Err(e) = self.update_wlcg_hourly(hour_start, hour_end).await {
                    tracing::error!("Failed to aggregate WLCG hour {hour_start}: {e:?}");
                }
            }
            if self.enable_metrics {
                if let Err(e) = self.update_generic_hourly(hour_start, hour_end).await {
                    tracing::error!("Failed to aggregate generic hour {hour_start}: {e:?}");
                }
            }

            // Sleep until the next hour is complete.
            let next = hour_end + one_hour + self.wlcg_metrics_delay;
            let wait = (next - Utc::now())
                .to_std()
                .unwrap_or(StdDuration::from_secs(1));
            tokio::time::sleep(wait).await;
        }
    }

    #[tracing::instrument(name = "Update record count for database metrics", skip(self))]
    async fn update_record_count(&self) -> Result<(), anyhow::Error> {
        let num = sqlx::query_scalar!(r#"SELECT count(*) as "count!" FROM auditor_accounting;"#)
            .fetch_one(&self.db_pool)
            .await?;
        self.data.lock().unwrap().num_records = Some(num);
        Ok(())
    }

    async fn query_count_per_key(&self, key: &str) -> Result<HashMap<String, i64>, anyhow::Error> {
        let map = sqlx::query_as::<_, AggregatedColumns>(COUNT_PER_KEY_SQL)
            .bind(key)
            .fetch_all(&self.db_pool)
            .await?
            .into_iter()
            .map(AggregatedColumns::into_tuple)
            .collect();
        Ok(map)
    }

    #[tracing::instrument(name = "Update record count per site for database metrics", skip(self))]
    async fn update_record_count_per_site(&self) -> Result<(), anyhow::Error> {
        let Some(key) = &self.meta_key_site else {
            tracing::warn!("RecordCountPerSite requested but meta_key_site is not set");
            return Ok(());
        };
        let per_site = self.query_count_per_key(key).await?;
        self.data.lock().unwrap().num_records_per_site = Some(per_site);
        Ok(())
    }

    #[tracing::instrument(
        name = "Update record count per group for database metrics",
        skip(self)
    )]
    async fn update_record_count_per_group(&self) -> Result<(), anyhow::Error> {
        let Some(key) = &self.meta_key_group else {
            tracing::warn!("RecordCountPerGroup requested but meta_key_group is not set");
            return Ok(());
        };
        let per_group = self.query_count_per_key(key).await?;
        self.data.lock().unwrap().num_records_per_group = Some(per_group);
        Ok(())
    }

    #[tracing::instrument(name = "Update record count per user for database metrics", skip(self))]
    async fn update_record_count_per_user(&self) -> Result<(), anyhow::Error> {
        let Some(key) = &self.meta_key_user else {
            tracing::warn!("RecordCountPerUser requested but meta_key_user is not set");
            return Ok(());
        };
        let per_user = self.query_count_per_key(key).await?;
        self.data.lock().unwrap().num_records_per_user = Some(per_user);
        Ok(())
    }

    #[tracing::instrument(name = "Aggregate last complete hour for WLCG dashboard", skip(self))]
    async fn update_wlcg_hourly(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        let rows: Vec<HourlyRow> = sqlx::query_as(HOURLY_SQL)
            .bind(&self.meta_key_site)
            .bind(&self.meta_key_user)
            .bind(&self.meta_key_submithost)
            .bind(&self.meta_key_group)
            .bind(&self.meta_key_infrastructure)
            .bind(start)
            .bind(end)
            .bind(&self.benchmark_name)
            .bind(&self.component_name_cores)
            .bind(&self.component_name_total_cpu)
            .fetch_all(&self.db_pool)
            .await?;

        tracing::info!(
            %start,
            %end,
            label_sets = rows.len(),
            records = rows.iter().map(|r| r.records).sum::<i64>(),
            "Aggregated hourly WLCG dashboard metrics"
        );

        self.data.lock().unwrap().hourly = Some(HourlySnapshot {
            // Stamp late inside the hour: within any hour-aligned time range,
            // and recent enough for Prometheus to accept without config changes.
            hour: start
                + chrono::Duration::try_minutes(SAMPLE_OFFSET_MINUTES)
                    .expect("This should never fail"),
            rows,
        });
        Ok(())
    }

    #[tracing::instrument(
        name = "Aggregate last complete hour for generic dashboard",
        skip(self)
    )]
    async fn update_generic_hourly(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        let rows: Vec<GenericRow> = sqlx::query_as(GENERIC_SQL)
            .bind(&self.excluded_meta_keys)
            .bind(&self.excluded_components)
            .bind(start)
            .bind(end)
            .fetch_all(&self.db_pool)
            .await?;

        tracing::info!(
            %start,
            %end,
            series = rows.len(),
            "Aggregated generic dashboard metrics"
        );

        self.data.lock().unwrap().generic = Some(HourlySnapshot {
            hour: start
                + chrono::Duration::try_minutes(SAMPLE_OFFSET_MINUTES)
                    .expect("This should never fail"),
            rows,
        });
        Ok(())
    }

    fn hourly_family(
        &self,
        name: &str,
        help: &str,
        snap: &HourlySnapshot<HourlyRow>,
        value: impl Fn(&HourlyRow) -> f64,
    ) -> MetricFamily {
        let ts_ms = snap.hour.timestamp_millis();

        let metrics: Vec<Metric> = snap
            .rows
            .iter()
            .map(|r| {
                let mut g = Gauge::default();
                g.set_value(value(r));
                let mut m = Metric::default();
                m.set_label(
                    vec![
                        label_pair("group", &r.group),
                        label_pair("infrastructure", &r.infrastructure),
                        label_pair("site", &r.site),
                        label_pair("submithost", &r.submithost),
                        label_pair("user", &r.user),
                    ]
                    .into(),
                );
                m.set_gauge(g);
                // Timestamp = start of hour + SAMPLE_OFFSET_MINUTES.
                m.set_timestamp_ms(ts_ms);
                m
            })
            .collect();

        let mut fam = MetricFamily::default();
        fam.set_name(name.to_string());
        fam.set_help(help.to_string());
        fam.set_field_type(MetricType::GAUGE);
        fam.set_metric(metrics.into());
        fam
    }

    #[tracing::instrument(
        name = "Turning database metrics into gauges",
        skip(self),
        level = "debug"
    )]
    fn get_metrics(&self) -> Result<Vec<MetricFamily>, anyhow::Error> {
        let mut out = vec![];
        let data = self.data.lock().unwrap();

        // Legacy metrics (only populated when enable_wlcg_metrics is false).
        if let Some(num_records) = data.num_records {
            let gauge = IntGauge::new(
                "num_records_database",
                "Number of records in the Auditor database",
            )?;
            gauge.set(num_records);
            out.extend(gauge.collect());
        }

        if let Some(ref m) = data.num_records_per_site {
            out.extend(per_key_gauge("num_records_database_per_site", "site", m)?);
        }
        if let Some(ref m) = data.num_records_per_group {
            out.extend(per_key_gauge(
                "num_records_database_per_group",
                "group_id",
                m,
            )?);
        }
        if let Some(ref m) = data.num_records_per_user {
            out.extend(per_key_gauge(
                "num_records_database_per_user",
                "user_id",
                m,
            )?);
        }

        let max_age =
            chrono::Duration::try_minutes(MAX_EXPOSE_AGE_MINUTES).expect("This should never fail");

        // Generic dashboard.
        if let Some(snap) = data
            .generic
            .as_ref()
            .filter(|s| !s.rows.is_empty() && Utc::now() - s.hour <= max_age)
        {
            out.push(generic_family(snap));
        }

        // WLCG dashboard.
        if let Some(snap) = data
            .hourly
            .as_ref()
            .filter(|s| !s.rows.is_empty() && Utc::now() - s.hour <= max_age)
        {
            out.push(self.hourly_family(
                "auditor_core_hours",
                "Wall-clock core hours of records stopped in the hour",
                snap,
                |r| r.core_hours,
            ));
            out.push(self.hourly_family(
                "auditor_score_hours",
                "Wall-clock work (score-weighted core hours) in the hour",
                snap,
                |r| r.score_hours,
            ));
            out.push(self.hourly_family(
                "auditor_cpu_hours",
                "CPU hours of records stopped in the hour",
                snap,
                |r| r.cpu_hours,
            ));
            out.push(self.hourly_family(
                "auditor_cpu_score_hours",
                "CPU work (score-weighted CPU hours) in the hour",
                snap,
                |r| r.cpu_score_hours,
            ));
            out.push(self.hourly_family(
                "auditor_records",
                "Number of records stopped in the hour",
                snap,
                |r| r.records as f64,
            ));
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
