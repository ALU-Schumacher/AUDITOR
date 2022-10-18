// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

mod auditorsender;
mod database;
mod sacctcaller;
mod shutdown;

use std::time::Duration;

use auditor::telemetry::{get_subscriber, init_subscriber};
use color_eyre::eyre::Result;
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
        "Running slurm collector",
        %run_id,
    );
    let _span_guard = span.enter();

    let frequency = Duration::from_secs(10);
    let database_path = "sqlite://delete_me.sql";

    let (record_send, record_recv) = mpsc::channel(1024);
    let (shutdown_send, mut shutdown_recv) = mpsc::unbounded_channel();
    let (notify_sacctcaller_send, notify_sacctcaller_recv) = broadcast::channel(12);
    let (notify_auditorsender_send, notify_auditorsender_recv) = broadcast::channel(12);
    let shutdown_sender = ShutdownSender::new(notify_sacctcaller_send, notify_auditorsender_send);

    let shutdown_sacctcaller = Shutdown::new(notify_sacctcaller_recv);
    let sacctcaller = SacctCaller::new(
        frequency,
        record_send,
        shutdown_send.clone(),
        shutdown_sacctcaller,
    );
    tokio::spawn(async move { sacctcaller.run().await });

    let shutdown_auditorsender = Shutdown::new(notify_auditorsender_recv);
    let auditorsender = AuditorSender::new(
        database_path,
        record_recv,
        shutdown_send,
        shutdown_auditorsender,
    )
    .await?;
    tokio::spawn(async move { auditorsender.run().await });

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

    Ok(())
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
