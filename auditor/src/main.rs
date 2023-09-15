// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use auditor::configuration::get_configuration;
use auditor::metrics::DatabaseMetricsWatcher;
use auditor::startup::run;
use auditor::telemetry::{get_subscriber, init_subscriber};
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Read in configuration
    let configuration = get_configuration().expect("Failed to read configuration.");

    // Set up logging
    let subscriber = get_subscriber("AUDITOR".into(), configuration.log_level, std::io::stdout);
    init_subscriber(subscriber);

    // Create a connection pool for the PostgreSQL database
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());

    // Start background task
    let db_metrics_watcher = DatabaseMetricsWatcher::new(connection_pool.clone(), &configuration)?;
    let db_metrics_watcher_task = db_metrics_watcher.clone();
    // TODO: Don't panic!
    tokio::spawn(async move {
        db_metrics_watcher_task.monitor().await.unwrap();
    });

    // Create a TcpListener for a given address and port
    let address = format!(
        "{}:{}",
        configuration.application.addr, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;

    // Start server
    run(listener, connection_pool, db_metrics_watcher)?.await?;

    Ok(())
}
