use crate::configuration::PrometheusMetricsOptions;
use actix_web_prom::{PrometheusMetrics, PrometheusMetricsBuilder};
use prometheus::{IntGaugeVec, Opts, Registry};
use std::collections::HashMap;

#[derive(Clone)]
pub struct PrometheusExporterConfig {
    pub prometheus_metrics: PrometheusMetrics,
    pub resource_metric: IntGaugeVec,
    pub priority_metric: IntGaugeVec,
}

impl PrometheusExporterConfig {
    #[tracing::instrument(name = "Initializing Prometheus exporter")]
    pub fn build() -> Result<PrometheusExporterConfig, anyhow::Error> {
        let registry = Registry::new();

        let resource_metric = IntGaugeVec::new(
            Opts::new("resource_usage", "Resource usage metrics"),
            &["group"],
        )?;

        let priority_metric =
            IntGaugeVec::new(Opts::new("priority", "Priority metrics"), &["group"])?;

        registry.register(Box::new(resource_metric.clone()))?;
        registry.register(Box::new(priority_metric.clone()))?;

        let prometheus_metrics = PrometheusMetricsBuilder::new("auditor")
            .registry(registry)
            .endpoint("/metrics")
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build Prometheus metrics middleware: {e}"))?;

        Ok(PrometheusExporterConfig {
            prometheus_metrics,
            resource_metric,
            priority_metric,
        })
    }

    pub async fn update_prometheus_metrics(
        &self,
        resources: &HashMap<String, f64>,
        priorities: &HashMap<String, i64>,
        metrics: &[PrometheusMetricsOptions],
    ) -> Result<(), anyhow::Error> {
        for metric in metrics.iter() {
            match metric {
                PrometheusMetricsOptions::ResourceUsage => {
                    for (resource, value) in resources {
                        self.resource_metric
                            .with_label_values(&[resource])
                            .set(*value as i64);
                    }
                }
                PrometheusMetricsOptions::Priority => {
                    for (priority, value) in priorities {
                        self.priority_metric
                            .with_label_values(&[priority])
                            .set(*value);
                    }
                }
            };
        }
        Ok(())
    }
}
