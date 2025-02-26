use std::env;
use std::sync::OnceLock;

//use auditor::domain::{RecordAdd, ValidName};
use auditor_client::AuditorClientBuilder;

mod config;
use config::{Config, load_configuration};
mod constants;
use constants::ensure_lazies;
mod database;
use database::Database;
mod record_collector;
use record_collector::{KapiCollector, run_record_collector};
mod merger;
use merger::run_merger;

use tokio::{signal, sync::broadcast};

static CONFIG: OnceLock<Config> = OnceLock::new();

fn init() -> anyhow::Result<()> {
    if CONFIG.get().is_some() {
        return Ok(());
    };

    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "config.yml"
    };
    if CONFIG.set(load_configuration(config_path)?).is_err() {
        return Ok(());
    };

    // Tracing
    let config = CONFIG.get().unwrap();
    tracing_subscriber::fmt()
        .with_max_level(config.log_level)
        .with_line_number(true)
        .pretty()
        //.compact()
        .init();
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ensure_lazies();
    init()?;
    println!("Loaded config {:?}", CONFIG.get());
    let config = CONFIG.get().unwrap();

    // Shutdown Channel
    // Create all receivers before anything can be sent
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
    let shutdown_rx1 = shutdown_tx.subscribe();
    let shutdown_rx2 = shutdown_tx.subscribe();

    // AuditorClient
    let client = if config.tls_config.use_tls {
        let tls_config = &config.tls_config;
        let _ = tls_config
            .validate_tls_paths()
            .map_err(|e| tracing::error!("Configuration error: {}", e));

        let ca_cert_path = tls_config.ca_cert_path.as_ref().unwrap();
        let client_key_path = tls_config.client_key_path.as_ref().unwrap();
        let client_cert_path = tls_config.client_cert_path.as_ref().unwrap();

        // Build client with TLS
        AuditorClientBuilder::new()
            .address(&config.auditor_addr, config.auditor_port)
            .timeout(config.auditor_timeout.num_seconds())
            .with_tls(client_cert_path, client_key_path, ca_cert_path)
            .build()
            .map_err(|e| tracing::error!("Error {:?}", e))
    } else {
        // Build client without TLS
        AuditorClientBuilder::new()
            .address(&config.auditor_addr, config.auditor_port)
            .timeout(config.auditor_timeout.num_seconds())
            .build()
            .map_err(|e| tracing::error!("Error {:?}", e))
    };

    // Prometheus Client
    let pclient = merger::build_pclient(
        &format!(
            "http://{}:{}",
            config.prometheus_addr.clone(),
            config.prometheus_port
        ),
        config.prometheus_timeout.to_std()?,
    )?;

    // Database
    let database = Database::new(
        &config.database_path.join("mqueue.db"),
        config.backlog_maxretries,
        config.backlog_interval.as_secs().try_into().unwrap(),
    )
    .await?;

    // This task will collect records from the Kubernetes API.
    // These will not have resource metrics.
    // Puts them into the database
    let collector = KapiCollector::new().await;
    run_record_collector(
        collector,
        database.clone(),
        shutdown_tx.clone(),
        shutdown_rx1,
    )?;

    // Will try to complete the records in the database with
    // resource metrics from Prometheus.
    // Will send the completed records to AUDITOR.
    run_merger(
        database,
        shutdown_tx.clone(),
        shutdown_rx2,
        client
            .clone()
            .expect("Error while setting up AuditorClientBuilder"),
        pclient,
    )?;

    // Shutdown
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = signal::ctrl_c() => {
            tracing::info!("CTRL-C received");
        },
        _ = sigterm.recv() => {
            tracing::info!("SIGTERM received");
        },
        _ = shutdown_rx.recv() => {
            tracing::info!("Shutdown signal from inside application received.");
        },
    }
    if let Err(e) = shutdown_tx.send(()) {
        tracing::error!("Could not send shutdown signal to tasks: {:?}", e);
    }
    // Drop local tx first, otherwise program will hang indefinitely.
    drop(shutdown_tx);
    // Will only yield when all senders are dropped,
    // effectively waiting for all tasks to finish.
    while shutdown_rx.recv().await != Err(broadcast::error::RecvError::Closed) {}
    //client.stop().await?;
    tracing::info!("Reached the end. Bye");
    Ok(())
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::ops::{Range, RangeBounds};
    use std::path::Path;
    use kube::api::{PostParams, WatchParams};
    use futures::{StreamExt, TryStreamExt};
    use anyhow::Result;

}
*/
