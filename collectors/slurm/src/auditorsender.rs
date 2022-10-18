// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use auditor::domain::Record;
use color_eyre::eyre::Result;
use tokio::sync::mpsc;

use crate::{database::Database, shutdown::Shutdown};

pub(crate) struct AuditorSender {
    database: Database,
    rx: mpsc::Receiver<Record>,
    _shutdown_notifier: mpsc::UnboundedSender<()>,
    shutdown: Option<Shutdown>,
}

impl<'a> AuditorSender {
    pub(crate) async fn new<S: AsRef<str>>(
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
    pub(crate) async fn run(mut self) {
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
    pub(crate) async fn handle_record(&self, record: Record) {
        tracing::debug!("Handling record: {:?}", record);
    }
}
