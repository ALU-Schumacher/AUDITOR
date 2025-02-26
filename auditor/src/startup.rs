// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::configuration::TLSParams;
use crate::metrics::{DatabaseMetricsWatcher, PrometheusExporterBuilder, PrometheusExporterConfig};
use crate::routes::{add, bulk_add, health_check, query_one_record, query_records, update};
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use actix_web_opentelemetry::{PrometheusMetricsHandler, RequestMetrics};
use opentelemetry::global;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

/// Configures and starts the HttpServer
pub fn run(
    addr: String,
    port: u16,
    db_pool: PgPool,
    db_watcher: DatabaseMetricsWatcher,
    tls_params: Option<TLSParams>,
) -> Result<Server, anyhow::Error> {
    let request_metrics: PrometheusExporterConfig = PrometheusExporterBuilder::new()
        .with_database_watcher(db_watcher)
        .build()?;
    global::set_meter_provider(request_metrics.provider);

    let db_pool = web::Data::new(db_pool);

    let app_config = move || {
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
            // Routes
            .route("/health_check", web::get().to(health_check))
            .service(
                web::resource("/record")
                    .route(web::post().to(add))
                    .route(web::put().to(update)),
            )
            .route("/record/{record_id}", web::get().to(query_one_record))
            // DB connection pool
            .service(
                web::resource("/records")
                    .route(web::post().to(bulk_add))
                    .route(web::get().to(query_records)),
            )
            .app_data(db_pool.clone())
    };

    let address = format!("{}:{}", addr, port);
    let listener = TcpListener::bind(address)?;

    let server = HttpServer::new(app_config).listen(listener)?;

    match tls_params {
        Some(params) if params.use_tls => {
            println!("********* AUDITOR running in TLS mode *********");

            match params.https_addr {
                Some(https_addr) => Ok(server
                    .bind_rustls_0_23((https_addr, params.https_port), params.config)?
                    .run()),
                _ => Ok(server
                    .bind_rustls_0_23((addr, params.https_port), params.config)?
                    .run()),
            }
        }
        _ => {
            println!("********* AUDITOR running without TLS *********");
            Ok(server.run())
        }
    }
}
