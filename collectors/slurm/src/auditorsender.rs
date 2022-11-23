// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::time::Duration;

use auditor::{client::AuditorClient, domain::RecordAdd};
use color_eyre::eyre::{Result, WrapErr};
use tokio::sync::{mpsc, oneshot};

use crate::{database::Database, shutdown::Shutdown, CONFIG};

pub(crate) struct AuditorSender {
    sender: QueuedSender,
    rx: mpsc::Receiver<RecordAdd>,
    _shutdown_notifier: mpsc::UnboundedSender<()>,
    shutdown: Option<Shutdown>,
    hold_till_shutdown: Option<mpsc::Sender<()>>,
}

impl<'a> AuditorSender {
    #[tracing::instrument(
        name = "Starting AuditorSender",
        skip(database, rx, shutdown_notifier, shutdown, channel, client)
    )]
    pub(crate) async fn run(
        database: Database,
        rx: mpsc::Receiver<RecordAdd>,
        shutdown_notifier: mpsc::UnboundedSender<()>,
        shutdown: Shutdown,
        channel: mpsc::Sender<()>,
        client: AuditorClient,
    ) -> Result<()> {
        let auditor_sender = AuditorSender {
            sender: QueuedSender::new(database, CONFIG.sender_frequency, client).await?,
            rx,
            _shutdown_notifier: shutdown_notifier,
            shutdown: Some(shutdown),
            hold_till_shutdown: Some(channel),
        };
        auditor_sender.run_internal().await?;

        Ok(())
    }

    async fn run_internal(mut self) -> Result<()> {
        tokio::spawn(async move {
            let mut shutdown = self.shutdown.take().expect("Definitely a bug.");

            while let Some(record) = tokio::select! {
                some_record = self.rx.recv() => { some_record }
                _ = shutdown.recv() => {
                    tracing::info!("AuditorSender received shutdown signal");
                    match self.sender.stop().await {
                        Ok(_) => {},
                        Err(e) => { tracing::error!("Stopping QueuedSender failed: {:?}", e); },
                    };
                    drop(self.hold_till_shutdown.take());
                    None
                },
            } {
                self.handle_record(record).await.unwrap();
            }
        });
        Ok(())
    }

    #[tracing::instrument(name = "Handling new record", skip(self))]
    async fn handle_record(&self, record: RecordAdd) -> Result<()> {
        tracing::debug!("Handling record: {:?}", record);
        self.sender.add_record(record).await
    }
}

pub(crate) struct QueuedSender {
    database: Database,
    shutdown_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
    shutdown_rx: Option<oneshot::Receiver<oneshot::Sender<()>>>,
    frequency: Duration,
    client: Option<AuditorClient>,
}

impl QueuedSender {
    pub(crate) async fn new(
        database: Database,
        frequency: Duration,
        client: AuditorClient,
    ) -> Result<QueuedSender> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let mut sender = QueuedSender {
            database,
            shutdown_tx: Some(shutdown_tx),
            shutdown_rx: Some(shutdown_rx),
            frequency,
            client: Some(client),
        };
        sender.run().await;
        Ok(sender)
    }

    pub(crate) async fn add_record(&self, record: RecordAdd) -> Result<()> {
        self.database.insert(record).await
    }

    #[tracing::instrument(name = "Stopping QueuedSender", skip(self))]
    pub(crate) async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let (tx, rx) = oneshot::channel();
            // shutdown_tx.send(tx).unwrap();
            match shutdown_tx.send(tx) {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("error: {:?}", e)
                }
            }
            rx.await.context("Shutting down QueuedSender failed.")?;
        }
        self.database.close().await;
        Ok(())
    }

    async fn run(&mut self) {
        let mut interval = tokio::time::interval(self.frequency);
        let mut shutdown_rx = self.shutdown_rx.take().expect("Bug.");
        let client = self.client.take().expect("Bug.");

        let database = self.database.clone();

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                tokio::select! {
                    _ = process_queue(&database, &client) => { },
                    res = &mut shutdown_rx => {
                        tracing::info!("QueuedSender received shutdown signal. Shutting down.");
                        // shutdown properly
                        // Report back shutdown
                        match res {
                            Ok(tx) => { tx.send(()).unwrap() },
                            Err(e) => { tracing::error!("Error: {:?}", e) },
                        }
                        break;
                    },
                }
            }
        });
    }
}

async fn process_queue(database: &Database, client: &AuditorClient) -> Result<()> {
    let entries = database.get_records().await?;
    for (id, record) in entries {
        tracing::info!("Sending record {}", id);
        match client.add(&record).await {
            Ok(_) => {
                tracing::info!("Successfully sent record {}", id);
                database.delete(id).await?;
            }
            Err(e) => {
                tracing::error!(
                    "Failed sending record {} to Auditor instance. Requeuing. Error: {:?}",
                    id,
                    e
                );
            }
        }
    }
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    Ok(())
}
