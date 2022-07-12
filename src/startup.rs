// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::routes::{add, get, get_since, health_check, update};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

/// Configures and starts the HttpServer
pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            // Logging middleware
            .wrap(TracingLogger::default())
            // Routes
            .route("/health_check", web::get().to(health_check))
            .route("/add", web::post().to(add))
            .route("/update", web::post().to(update))
            .route("/get", web::get().to(get))
            .route("/get/{startstop}/since/{date}", web::get().to(get_since))
            // DB connection pool
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
