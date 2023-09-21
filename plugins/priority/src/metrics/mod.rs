use crate::configuration::PrometheusMetricsOptions;
use opentelemetry::sdk::metrics::MeterProvider;
use prometheus::Registry;
use prometheus::{IntGaugeVec, Opts};
use std::collections::HashMap;

#[derive(Clone)]
pub struct PrometheusExporterConfig {
    pub provider: MeterProvider,
    pub prom_registry: Registry,
    pub resource_metric: IntGaugeVec,
    pub priority_metric: IntGaugeVec,
}

impl PrometheusExporterConfig {
    #[tracing::instrument(name = "Initializing Prometheus exporter")]
    pub fn build() -> Result<PrometheusExporterConfig, anyhow::Error> {
        let prom_registry = Registry::new();

        let metrics_exporter = opentelemetry_prometheus::exporter()
            .with_registry(prom_registry.clone())
            .build()?;

        let resource_metric = IntGaugeVec::new(
            Opts::new("resource_usage", "Resource usage metrics"),
            &["group"],
        )?;

        let priority_metric =
            IntGaugeVec::new(Opts::new("priority", "Priority metrics"), &["group"])?;

        prom_registry.register(Box::new(resource_metric.clone()))?;
        prom_registry.register(Box::new(priority_metric.clone()))?;

        let provider = MeterProvider::builder()
            .with_reader(metrics_exporter)
            .build();

        Ok(PrometheusExporterConfig {
            provider,
            prom_registry,
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
