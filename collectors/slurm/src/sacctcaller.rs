// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::time::Duration;

use auditor::domain::Record;
use color_eyre::eyre::Result;
use fake::{Fake, Faker};
use tokio::sync::mpsc;

use crate::shutdown::Shutdown;

pub(crate) struct SacctCaller {
    frequency: Duration,
    tx: mpsc::Sender<Record>,
    _shutdown_notifier: mpsc::UnboundedSender<()>,
    shutdown: Option<Shutdown>,
}

impl SacctCaller {
    pub fn new(
        frequency: Duration,
        tx: mpsc::Sender<Record>,
        shutdown_notifier: mpsc::UnboundedSender<()>,
        shutdown: Shutdown,
    ) -> SacctCaller {
        SacctCaller {
            frequency,
            tx,
            _shutdown_notifier: shutdown_notifier,
            shutdown: Some(shutdown),
        }
    }

    #[tracing::instrument(name = "Starting SacctCaller", skip(self))]
    pub async fn run(mut self) {
        let mut shutdown = self.shutdown.take().expect("Definitely a bug.");
        let mut interval = tokio::time::interval(self.frequency);
        loop {
            interval.tick().await;
            tokio::select! {
                records = self.get_job_info() => {
                    match records {
                        Ok(records) => self.place_records_on_queue(records).await,
                        Err(e) => {
                            tracing::error!("something went wrong: {:?}", e);
                            continue
                        }
                    };
                },
                _ = shutdown.recv() => {
                    tracing::info!("SacctCaller received shutdown signal. Shutting down.");
                    // shutdown properly
                    break
                },
            }
        }
    }

    #[tracing::instrument(
        name = "Placing records on queue",
        level = "debug",
        skip(self, records)
    )]
    async fn place_records_on_queue(&self, records: Vec<Record>) {
        for record in records {
            let record_id = record.record_id.clone();
            if let Err(e) = self.tx.send(record).await {
                tracing::error!("Could not send record {:?} to queue: {:?}", record_id, e);
            }
        }
    }

    async fn get_job_info(&self) -> Result<Vec<Record>> {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let record: auditor::domain::RecordTest = Faker.fake();
        Ok(vec![record.try_into().unwrap()])
    }
}
