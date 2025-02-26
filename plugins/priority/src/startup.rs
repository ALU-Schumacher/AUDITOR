use crate::metrics::PrometheusExporterConfig;
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use actix_web_opentelemetry::{PrometheusMetricsHandler, RequestMetrics};
use opentelemetry::global;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

/// Configures and starts the HttpServer
pub async fn run(
    listener: TcpListener,
    request_metrics: PrometheusExporterConfig,
) -> Result<Server, std::io::Error> {
    global::set_meter_provider(request_metrics.provider);

    let server = HttpServer::new(move || {
        App::new()
            // Logging middleware
            .wrap(TracingLogger::default())
            .wrap(RequestMetrics::default())
            .route(
                "/metrics",
                web::get().to(PrometheusMetricsHandler::new(
                    request_metrics.prom_registry.clone(),
                )),
            )
    })
    .listen(listener)?
    .run();

    Ok(server)
}
