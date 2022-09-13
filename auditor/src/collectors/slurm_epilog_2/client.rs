// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anyhow::Context;
use bytes::BytesMut;
use std::{
    io::{Read, Write},
    net::TcpStream,
};
use uuid::Uuid;

use auditor::telemetry::{get_subscriber, init_subscriber};
use configuration::client::get_configuration;
use message::Message;

mod configuration;
mod message;

const NAME: &str = "AUDITOR-slurm-epilog-collector-client";

fn run() -> Result<(), anyhow::Error> {
    let config = get_configuration()?;
    let addr = config.get_addr();

    let message = Message::jobinfo_from_env().context(concat!(
        "Could not get Slurm job info from environment. ",
        "Make sure to run the client in the Slurm epilog context."
    ))?;

    tracing::debug!("Connecting to {}", addr);
    let mut stream = TcpStream::connect("127.0.0.1:4687")?;
    let local_addr = stream.local_addr()?;
    tracing::debug!("Connected to {}:{}", local_addr.ip(), local_addr.port());

    tracing::debug!("Serializing message and writing to TCP stream");
    stream.write_all(&message.pack())?;
    let _ = stream.flush();

    tracing::debug!("Receiving response from server");
    let mut buffer = [0; 1024];
    stream
        .read(&mut buffer)
        .context("Reading response from server failed")?;

    match Message::unpack(&BytesMut::from(&buffer[..]))
        .context("Deserializing message from server failed.")?
    {
        Message::Ok => (),
        Message::Error { msg } => return Err(anyhow::anyhow!("Server said no: {}", msg)),
        _ => {
            return Err(anyhow::anyhow!(
                "Received unacceptable message from server."
            ))
        }
    }

    Ok(())
}

fn main() {
    let subscriber = get_subscriber(NAME.into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let run_id = Uuid::new_v4();
    let span = tracing::info_span!(
        "Running slurm epilog collector client",
        %run_id,
    );
    let _span_guard = span.enter();

    if let Err(e) = run() {
        tracing::error!("Failed to execute {}: {:?}", NAME, e);
    }
}
