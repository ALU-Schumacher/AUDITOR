// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use serde::{Deserialize, de};
use std::str::FromStr;
use tracing::{Subscriber, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{
    EnvFilter, Registry, filter::LevelFilter, fmt::MakeWriter, layer::SubscriberExt,
};

/// Compose multiple layers into a `tracing`'s subscriber.
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: LevelFilter,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    //let env_filter =
    //    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let env_filter = EnvFilter::from_default_env().add_directive(env_filter.into());
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Register a subscriber as global default for processing span data.
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

pub fn deserialize_log_level<'de, D>(deserializer: D) -> Result<LevelFilter, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    LevelFilter::from_str(&s.to_lowercase()).map_err(de::Error::custom)
}
