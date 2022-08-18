// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use prometheus::Registry;

mod database;
pub use database::*;

pub struct PrometheusExporterBuilder<F> {
    db_watcher: Option<DatabaseMetricsWatcher>,
    should_render_metrics: F,
}

impl<F> PrometheusExporterBuilder<F>
where
    F: Fn(&actix_web::dev::ServiceRequest) -> bool + Send + Clone,
{
    pub fn new(should_render_metrics: F) -> PrometheusExporterBuilder<F> {
        PrometheusExporterBuilder {
            db_watcher: None,
            should_render_metrics,
        }
    }

    pub fn with_database_watcher(mut self, db_watcher: DatabaseMetricsWatcher) -> Self {
        self.db_watcher = Some(db_watcher);
        self
    }

    #[tracing::instrument(name = "Initializing Prometheus exporter", skip(self))]
    pub fn build(self) -> Result<actix_web_opentelemetry::RequestMetrics<F>, anyhow::Error> {
        let prom_registry = Registry::new();

        if let Some(db_watcher) = self.db_watcher {
            prom_registry.register(std::boxed::Box::new(db_watcher))?;
        }

        let metrics_exporter = opentelemetry_prometheus::exporter()
            .with_registry(prom_registry)
            .init();

        Ok(actix_web_opentelemetry::RequestMetrics::new(
            opentelemetry::global::meter("auditor_http_tracing"),
            Some(self.should_render_metrics),
            Some(metrics_exporter),
        ))
    }
}
