// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::fmt;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

/// Compose multiple layers into a `tracing`'s subscriber.
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
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

/// Log level options for AUDITOR and collectors                                                                                                                                          

#[derive(serde::Deserialize, Debug, Clone)]
pub enum LogLevel {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::TRACE => "trace",
            LogLevel::DEBUG => "debug",
            LogLevel::INFO => "info",
            LogLevel::WARN => "warn",
            LogLevel::ERROR => "error",
        }
    }
}

impl TryFrom<String> for LogLevel {
    type Error = String;
    fn try_from(log_string: String) -> Result<Self, Self::Error> {
        match log_string.to_lowercase().as_str() {
            "trace" => Ok(Self::TRACE),
            "debug" => Ok(Self::DEBUG),
            "info" => Ok(Self::INFO),
            "warn" => Ok(Self::WARN),
            "error" => Ok(Self::ERROR),
            _ => Err("Not a supported log level. Please check your spelling. 
					Setting INFO as default log level"
                .to_string()),
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LogLevel::TRACE => write!(f, "trace"),
            LogLevel::DEBUG => write!(f, "debug"),
            LogLevel::INFO => write!(f, "info"),
            LogLevel::WARN => write!(f, "warn"),
            LogLevel::ERROR => write!(f, "error"),
        }
    }
}
