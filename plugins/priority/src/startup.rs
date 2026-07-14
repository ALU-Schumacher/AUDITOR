use crate::metrics::PrometheusExporterConfig;
use actix_web::dev::Server;
use actix_web::{App, HttpServer};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

/// Configures and starts the HttpServer
pub async fn run(
    listener: TcpListener,
    request_metrics: PrometheusExporterConfig,
) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(move || {
        App::new()
            // Logging middleware
            .wrap(TracingLogger::default())
            .wrap(request_metrics.prometheus_metrics.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
