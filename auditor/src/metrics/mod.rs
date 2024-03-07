// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use opentelemetry_sdk::metrics::SdkMeterProvider;
use prometheus::Registry;

mod database;
pub use database::*;

pub struct PrometheusExporterConfig {
    pub provider: SdkMeterProvider,
    pub prom_registry: Registry,
}

pub struct PrometheusExporterBuilder {
    db_watcher: Option<DatabaseMetricsWatcher>,
}

impl PrometheusExporterBuilder {
    pub fn new() -> PrometheusExporterBuilder {
        PrometheusExporterBuilder { db_watcher: None }
    }

    pub fn with_database_watcher(mut self, db_watcher: DatabaseMetricsWatcher) -> Self {
        self.db_watcher = Some(db_watcher);
        self
    }

    #[tracing::instrument(name = "Initializing Prometheus exporter", skip(self))]
    pub fn build(self) -> Result<PrometheusExporterConfig, anyhow::Error> {
        let prom_registry = Registry::new();

        if let Some(db_watcher) = self.db_watcher {
            prom_registry.register(std::boxed::Box::new(db_watcher))?;
        }

        let metrics_exporter = opentelemetry_prometheus::exporter()
            .with_registry(prom_registry.clone())
            .build()?;

        let provider = SdkMeterProvider::builder()
            .with_reader(metrics_exporter)
            .build();

        Ok(PrometheusExporterConfig {
            provider,
            prom_registry,
        })
    }
}

impl Default for PrometheusExporterBuilder {
    fn default() -> Self {
        Self::new()
    }
}
