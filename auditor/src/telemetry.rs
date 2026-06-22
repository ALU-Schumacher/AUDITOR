// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use serde::{Deserialize, de};
use std::str::FromStr;
use tracing::{Subscriber, subscriber::set_global_default};
use tracing_appender::non_blocking::WorkerGuard;
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
    file_config: Option<(impl AsRef<std::path::Path>, &str)>,
) -> (Box<dyn Subscriber + Send + Sync>, Vec<WorkerGuard>)
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    //let env_filter =
    //    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let env_filter = EnvFilter::from_default_env().add_directive(env_filter.into());
    let stdout_formatting_layer = BunyanFormattingLayer::new(name.clone(), sink);

    let base = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(stdout_formatting_layer);

    match file_config {
        Some((log_dir, log_file_prefix)) => {
            std::fs::create_dir_all(&log_dir).expect("Failed to create log directory");

            let file_appender = tracing_appender::rolling::daily(log_dir, log_file_prefix);
            let (non_blocking_file, file_guard) = tracing_appender::non_blocking(file_appender);
            let file_formatting_layer = BunyanFormattingLayer::new(name, non_blocking_file);

            let subscriber = base.with(file_formatting_layer);
            (Box::new(subscriber), vec![file_guard])
        }
        None => (Box::new(base), vec![]),
    }
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
