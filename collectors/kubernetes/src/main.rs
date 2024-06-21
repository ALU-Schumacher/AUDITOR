use std::env;
use std::sync::OnceLock;
use std::time::Duration;

//use auditor::domain::{RecordAdd, ValidName};
use auditor_client::AuditorClientBuilder;

mod config;
use config::{load_configuration, Config};
mod constants;
use constants::ensure_lazies;
mod database;
use database::Database;
mod record_collector;
use record_collector::{run_record_collector, KapiCollector};
mod merger;
use merger::run_merger;

use tokio::{signal, sync::broadcast};

static CONFIG: OnceLock<Config> = OnceLock::new();

fn init() {
    if CONFIG.get().is_some() {
        return;
    };

    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "config.yml"
    };
    if CONFIG.set(load_configuration(config_path)).is_err() {
        return;
    };

    // Tracing
    let config = CONFIG.get().unwrap();
    tracing_subscriber::fmt()
        .with_max_level(config.log_level)
        .with_line_number(true)
        .pretty()
        //.compact()
        //.with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ensure_lazies();
    init();
    println!("Loaded config {:?}", CONFIG.get());
    let config = CONFIG.get().unwrap();

    // Shutdown Channel
    // Create all receivers before anything can be sent
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
    let shutdown_rx1 = shutdown_tx.subscribe();
    let shutdown_rx2 = shutdown_tx.subscribe();

    // Make AUDITOR Client
    let client = AuditorClientBuilder::new()
        .address(&config.auditor_addr, config.auditor_port)
        .build()?;

    // Prometheus Client
    let pclient = merger::build_pclient(
        &format!(
            "http://{}:{}",
            config.prometheus_addr.clone(),
            config.prometheus_port
        ),
        Duration::from_secs(30),
    )?;
    //let pclient = PClient::try_from(format!(
    //    "http://{}:{}",
    //    config.prometheus_addr.clone(),
    //    config.prometheus_port
    //))?;

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
        client.clone(),
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
