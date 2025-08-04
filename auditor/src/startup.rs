// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::archive::ArchiveService;
use crate::configuration::{ArchivalConfig, TLSParams};
use crate::metrics::{DatabaseMetricsWatcher, PrometheusExporterBuilder, PrometheusExporterConfig};
use crate::middleware::rbac;
use crate::routes::{add, bulk_add, health_check, query_one_record, query_records, update};
use actix_tls::accept::rustls_0_23::TlsStream;
use actix_web::dev::Server;
use actix_web::middleware::from_fn;
use actix_web::{App, HttpResponse, HttpServer, web};
use actix_web::{dev::Extensions, rt::net::TcpStream};
use actix_web_opentelemetry::{PrometheusMetricsHandler, RequestMetrics};
use casbin::{CoreApi, DefaultModel, Enforcer, FileAdapter};
use opentelemetry::global;
use sqlx::PgPool;
use std::{any::Any, net::SocketAddr, sync::Arc};
use tracing::info;
use tracing_actix_web::TracingLogger;

/// Configures and starts the HttpServer
#[allow(clippy::too_many_arguments)]
pub async fn run(
    addrs: Vec<String>,
    port: u16,
    web_workers: usize,
    db_pool: PgPool,
    db_watcher: DatabaseMetricsWatcher,
    tls_params: Option<TLSParams>,
    enforce_rbac: bool,
    ignore_record_exists_error: bool,
    archival_config: Option<ArchivalConfig>,
) -> Result<Server, anyhow::Error> {
    let request_metrics: PrometheusExporterConfig = PrometheusExporterBuilder::new()
        .with_database_watcher(db_watcher)
        .build()?;
    global::set_meter_provider(request_metrics.provider);

    if let Some(archival_config) = archival_config {
        let archival_service = ArchiveService::new(db_pool.clone(), archival_config);
        archival_service.start_scheduler().await?;
    }

    let db_pool = web::Data::new(db_pool);

    let enforcer_settings = if enforce_rbac {
        let m = DefaultModel::from_file("model.conf").await.unwrap();
        let adapter = FileAdapter::new("policy.csv");
        let mut enforcer = Enforcer::new(m, adapter).await.unwrap();
        enforcer.enable_auto_save(true);
        Some(Arc::new(enforcer))
    } else {
        None
    };

    let app_config = move || {
        let enforcer_data = web::Data::new(enforcer_settings.clone());
        let enforce_rbac_data = web::Data::new(enforce_rbac);
        let ignore_record_exists_error_data = web::Data::new(ignore_record_exists_error);

        App::new()
            // Logging middleware
            .app_data(enforcer_data)
            .app_data(enforce_rbac_data)
            .app_data(ignore_record_exists_error_data)
            .wrap(TracingLogger::default())
            .wrap(RequestMetrics::default())
            .wrap(from_fn(rbac))
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
            .default_service(web::route().to(|| async {
                HttpResponse::NotFound().body("The requested resource was not found. 404 Not Found")
            }))
    };

    match tls_params {
        Some(params) if params.use_tls => {
            println!("********* AUDITOR running in TLS mode *********");

            let mut server = HttpServer::new(app_config)
                .workers(web_workers)
                .on_connect(get_client_cert);

            for addr in &addrs {
                let address = format!("{addr}:{port}");
                server = server.bind(&address)?;
            }

            match params.https_addr {
                Some(https_addrs) => {
                    for https_addr in https_addrs {
                        server = server.bind_rustls_0_23(
                            (https_addr, params.https_port),
                            params.config.clone(),
                        )?
                    }
                    Ok(server.run())
                }
                _ => {
                    for addr in addrs {
                        server = server
                            .bind_rustls_0_23((addr, params.https_port), params.config.clone())?
                    }
                    Ok(server.run())
                }
            }
        }
        _ => {
            println!("********* AUDITOR running without TLS *********");

            let mut server = HttpServer::new(app_config).workers(web_workers);

            for addr in &addrs {
                let address = format!("{addr}:{port}");
                server = server.bind(&address)?;
            }
            Ok(server.run())
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct ConnectionInfo {
    bind: SocketAddr,
    peer: SocketAddr,
    ttl: Option<u32>,
}

fn get_client_cert(connection: &dyn Any, data: &mut Extensions) {
    if let Some(tls_socket) = connection.downcast_ref::<TlsStream<TcpStream>>() {
        info!("TLS on_connect");

        let (socket, tls_session) = tls_socket.get_ref();

        data.insert(ConnectionInfo {
            bind: socket.local_addr().unwrap(),
            peer: socket.peer_addr().unwrap(),
            ttl: socket.ttl().ok(),
        });

        if let Some(certs) = tls_session.peer_certificates() {
            info!("client certificate found");

            // insert a `rustls::Certificate` into request data
            data.insert(certs.first().unwrap().clone());
        }
    } else if let Some(socket) = connection.downcast_ref::<TcpStream>() {
        info!("plaintext on_connect");

        data.insert(ConnectionInfo {
            bind: socket.local_addr().unwrap(),
            peer: socket.peer_addr().unwrap(),
            ttl: socket.ttl().ok(),
        });
    } else {
        unreachable!("socket should be TLS or plaintext");
    }
}
