// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::time::Duration;

use auditor::domain::RecordAdd;
use color_eyre::eyre::Result;
use fake::{Fake, Faker};
use tokio::sync::mpsc;

use crate::shutdown::Shutdown;

pub(crate) struct SacctCaller {
    frequency: Duration,
    tx: mpsc::Sender<RecordAdd>,
    _shutdown_notifier: mpsc::UnboundedSender<()>,
    shutdown: Option<Shutdown>,
    hold_till_shutdown: Option<mpsc::Sender<()>>,
}

impl SacctCaller {
    #[tracing::instrument(
        name = "Starting SacctCaller",
        skip(tx, shutdown_notifier, shutdown, channel)
    )]
    pub(crate) async fn run(
        frequency: Duration,
        tx: mpsc::Sender<RecordAdd>,
        shutdown_notifier: mpsc::UnboundedSender<()>,
        shutdown: Shutdown,
        channel: mpsc::Sender<()>,
    ) {
        let sacctcaller = SacctCaller {
            frequency,
            tx,
            _shutdown_notifier: shutdown_notifier,
            shutdown: Some(shutdown),
            hold_till_shutdown: Some(channel),
        };
        sacctcaller.run_internal().await;
    }

    async fn run_internal(mut self) {
        let mut shutdown = self.shutdown.take().expect("Definitely a bug.");

        tokio::spawn(async move {
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
                        drop(self.hold_till_shutdown.take());
                        break
                    },
                }
            }
        });
    }

    #[tracing::instrument(
        name = "Placing records on queue",
        level = "debug",
        skip(self, records)
    )]
    async fn place_records_on_queue(&self, records: Vec<RecordAdd>) {
        for record in records {
            let record_id = record.record_id.clone();
            if let Err(e) = self.tx.send(record).await {
                tracing::error!("Could not send record {:?} to queue: {:?}", record_id, e);
            }
        }
    }

    #[tracing::instrument(name = "Calling sacct and parsing output", skip(self))]
    async fn get_job_info(&self) -> Result<Vec<RecordAdd>> {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let record: auditor::domain::RecordTest = Faker.fake();
        Ok(vec![record.try_into().unwrap()])
    }
}
// let cmd_out = Command::new("/usr/bin/sacct")
//        .arg("-a")
//        .arg("-j")
//        .arg(job_id.to_string())
//        .arg("--format")
//        .arg(keys.iter().map(|k| k.0.clone()).join(","))
//        .arg("--noconvert")
//        .arg("--noheader")
//        .arg("-P")
//        .output()
//        .await?
//        .stdout;
// #[tracing::instrument(name = "Getting Slurm job info via scontrol")]
// fn get_slurm_job_info(job_id: u64) -> Result<Job, Error> {
//     Ok(std::str::from_utf8(
//         &Command::new("/usr/bin/scontrol")
//             .arg("show")
//             .arg("job")
//             .arg(job_id.to_string())
//             .arg("--details")
//             .output()?
//             .stdout,
//     )?
//     .split_whitespace()
//     .filter_map(|s| {
//         if let Some((k, v)) = s.split_once('=') {
//             Some((k.to_string(), v.to_string()))
//         } else {
//             None
//         }
//     })
//     .collect())
// }
