// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

mod auditorsender;
mod configuration;
mod database;
mod sacctcaller;
mod shutdown;

use std::time::Duration;

use auditor::{
    client::AuditorClientBuilder,
    telemetry::{get_subscriber, init_subscriber},
};
use color_eyre::eyre::{eyre, Result};
use sacctcaller::SacctCaller;
use shutdown::{Shutdown, ShutdownSender};
use tokio::{
    signal,
    sync::{broadcast, mpsc},
};
use uuid::Uuid;

use crate::auditorsender::AuditorSender;

const NAME: &str = "AUDITOR-slurm-collector";

// # CONFIGURATION TODOS:
//
// * SacctCaller frequency (std::time::Duration)
// * database_path (AsRef<Path>)
#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = get_subscriber(NAME.into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let run_id = Uuid::new_v4();
    let span = tracing::info_span!(
        "Slurm collector",
        %run_id,
    );
    let _span_guard = span.enter();

    let config = configuration::get_configuration()?;

    tracing::debug!(?config, "Loaded config");

    // Configs
    let frequency = Duration::from_secs(10);
    let sender_frequency = Duration::from_secs(1);
    let database_path = "sqlite://testdb.db";

    // Channels
    let (final_shutdown_tx, mut final_shutdown_rx) = mpsc::channel(1);
    let (record_send, record_recv) = mpsc::channel(1024);
    let (shutdown_send, mut shutdown_recv) = mpsc::unbounded_channel();
    let (notify_sacctcaller_send, notify_sacctcaller_recv) = broadcast::channel(12);
    let (notify_auditorsender_send, notify_auditorsender_recv) = broadcast::channel(12);

    // Shutdown
    let shutdown_sender = ShutdownSender::new()
        .with_sender(notify_sacctcaller_send)
        .with_sender(notify_auditorsender_send);

    // SacctCaller
    SacctCaller::run(
        frequency,
        record_send,
        shutdown_send.clone(),
        Shutdown::new(notify_sacctcaller_recv),
        final_shutdown_tx.clone(),
    )
    .await;

    // AuditorClient
    let client = AuditorClientBuilder::new()
        .address(&config.addr, config.port)
        .build()
        .map_err(|e| eyre!("Error {:?}", e))?;

    // AuditorSender
    AuditorSender::run(
        database_path,
        record_recv,
        shutdown_send,
        Shutdown::new(notify_auditorsender_recv),
        sender_frequency,
        final_shutdown_tx.clone(),
        client,
    )
    .await?;

    tokio::select! {
        _ = signal::ctrl_c() => {
            tracing::info!("CTRL-C recieved");
        },
        _ = shutdown_recv.recv() => {
            tracing::info!("Shutdown signal from inside application received.");
        },
    }

    if let Err(e) = shutdown_sender.shutdown() {
        tracing::error!("Could not send shutdown signal to tasks: {:?}", e);
    }

    // Drop local tx first, otherwise program will hang indefinitely.
    drop(final_shutdown_tx);
    // Will only yield when all tx channels are closed, effectively waiting for all tasks to finish.
    let _ = final_shutdown_rx.recv().await;
    Ok(())
}
