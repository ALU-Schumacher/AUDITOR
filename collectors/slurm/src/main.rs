// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use auditor::{
    domain::Record,
    telemetry::{get_subscriber, init_subscriber},
};
use color_eyre::eyre::Result;
use fake::{Fake, Faker};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};
use std::{str::FromStr, time::Duration};
use tokio::{
    signal,
    sync::{broadcast, mpsc},
};
use uuid::Uuid;

const NAME: &str = "AUDITOR-slurm-collector";

pub struct Shutdown {
    shutdown: bool,
    notify: broadcast::Receiver<()>,
}

pub struct ShutdownSender {
    notify_sacctcaller: broadcast::Sender<()>,
    notify_auditorsender: broadcast::Sender<()>,
}

impl ShutdownSender {
    pub fn new(
        notify_sacctcaller: broadcast::Sender<()>,
        notify_auditorsender: broadcast::Sender<()>,
    ) -> ShutdownSender {
        ShutdownSender {
            notify_sacctcaller,
            notify_auditorsender,
        }
    }

    #[tracing::instrument(
        name = "Sending shutdown signal to SacctCaller and AuditorSender",
        skip(self)
    )]
    pub fn shutdown(&self) -> Result<()> {
        self.notify_sacctcaller.send(())?;
        self.notify_auditorsender.send(())?;
        Ok(())
    }
}

impl Shutdown {
    fn new(notify: broadcast::Receiver<()>) -> Shutdown {
        Shutdown {
            shutdown: false,
            notify,
        }
    }

    /// Receive the shutdown notice, waiting if necessary.
    async fn recv(&mut self) {
        if self.shutdown {
            return;
        }

        let _ = self.notify.recv().await;

        self.shutdown = true;
    }
}

struct SacctCaller {
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
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let record: auditor::domain::RecordTest = Faker.fake();
        Ok(vec![record.try_into().unwrap()])
    }
}

struct Database {
    db_pool: SqlitePool,
}

impl Database {
    async fn new<S: AsRef<str>>(path: S) -> Result<Database> {
        let db_pool = SqlitePool::connect_with(
            SqliteConnectOptions::from_str(path.as_ref())?
                .journal_mode(SqliteJournalMode::Wal)
                .create_if_missing(true),
        )
        .await?;
        tracing::debug!("Migrating database");
        sqlx::migrate!().run(&db_pool).await?;
        Ok(Database { db_pool })
    }

    async fn _insert(&self, _record: Record) -> Result<()> {
        // let record_id = record.record_id.clone();
        // let record = bincode::serialize(&record);
        // sqlx::query!(r#"INSERT INTO records (id, record) VALUES (record_id, record)"#)
        //     .execute(&self.db_pool)
        //     .await;
        Ok(())
    }

    #[tracing::instrument(name = "Closing database connection", level = "info", skip(self))]
    async fn close(&self) {
        self.db_pool.close().await
    }
}

pub struct AuditorSender {
    database: Database,
    rx: mpsc::Receiver<Record>,
    _shutdown_notifier: mpsc::UnboundedSender<()>,
    shutdown: Option<Shutdown>,
}

impl<'a> AuditorSender {
    pub async fn new<S: AsRef<str>>(
        database_path: S,
        rx: mpsc::Receiver<Record>,
        shutdown_notifier: mpsc::UnboundedSender<()>,
        shutdown: Shutdown,
    ) -> Result<AuditorSender> {
        Ok(AuditorSender {
            database: Database::new(database_path).await?,
            rx,
            _shutdown_notifier: shutdown_notifier,
            shutdown: Some(shutdown),
        })
    }

    #[tracing::instrument(name = "Starting AuditorSender", skip(self))]
    pub async fn run(mut self) {
        let mut shutdown = self.shutdown.take().expect("Definitely a bug.");
        
        while let Some(record) = tokio::select! {
                some_record = self.rx.recv() => { some_record }
                _ = shutdown.recv() => {
                    tracing::info!("AuditorSender received shutdown signal");
                    self.database.close().await;
                    None
                },
            } {
            self.handle_record(record).await;
        }
    }

    #[tracing::instrument(name = "Handling new record", skip(self))]
    pub async fn handle_record(&self, record: Record) {
        tracing::debug!("Handling record: {:?}", record);
    }
}

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
