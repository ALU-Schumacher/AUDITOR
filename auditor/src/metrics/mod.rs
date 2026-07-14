// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//use opentelemetry_sdk::metrics::SdkMeterProvider;
use actix_web_prom::{PrometheusMetrics, PrometheusMetricsBuilder};
use prometheus::Registry;

mod database;
pub use database::*;

pub struct PrometheusExporterBuilder {
    db_watcher: Option<DatabaseMetricsWatcher>,
    namespace: String,
    endpoint: String,
}

impl PrometheusExporterBuilder {
    pub fn new() -> PrometheusExporterBuilder {
        PrometheusExporterBuilder {
            db_watcher: None,
            namespace: "auditor".to_string(),
            endpoint: "/metrics".to_string(),
        }
    }

    pub fn with_database_watcher(mut self, db_watcher: DatabaseMetricsWatcher) -> Self {
        self.db_watcher = Some(db_watcher);
        self
    }

    #[tracing::instrument(name = "Initializing Prometheus exporter", skip(self))]
    pub fn build(self) -> Result<PrometheusMetrics, anyhow::Error> {
        let registry = Registry::new();

        if let Some(db_watcher) = self.db_watcher {
            registry.register(std::boxed::Box::new(db_watcher))?;
        }

        PrometheusMetricsBuilder::new(&self.namespace)
            .registry(registry)
            .endpoint(&self.endpoint)
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build Prometheus metrics middleware: {e}"))
    }
}

impl Default for PrometheusExporterBuilder {
    fn default() -> Self {
        Self::new()
    }
}
