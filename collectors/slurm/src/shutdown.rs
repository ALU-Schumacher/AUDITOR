// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use color_eyre::eyre::Result;
use tokio::sync::broadcast;

pub(crate) struct Shutdown {
    shutdown: bool,
    notify: broadcast::Receiver<()>,
}

impl Shutdown {
    pub(crate) fn new(notify: broadcast::Receiver<()>) -> Shutdown {
        Shutdown {
            shutdown: false,
            notify,
        }
    }

    /// Receive the shutdown notice, waiting if necessary.
    pub(crate) async fn recv(&mut self) {
        if self.shutdown {
            return;
        }

        let _ = self.notify.recv().await;

        self.shutdown = true;
    }
}

pub(crate) struct ShutdownSender {
    notify: Vec<broadcast::Sender<()>>,
}

impl ShutdownSender {
    pub(crate) fn new() -> ShutdownSender {
        ShutdownSender { notify: vec![] }
    }

    pub(crate) fn with_sender(mut self, sender: broadcast::Sender<()>) -> ShutdownSender {
        self.notify.push(sender);
        self
    }

    #[tracing::instrument(
        name = "Sending shutdown signal to SacctCaller and AuditorSender",
        skip(self)
    )]
    pub(crate) fn shutdown(self) -> Result<()> {
        for notify in self.notify {
            notify.send(())?;
        }
        Ok(())
    }
}
