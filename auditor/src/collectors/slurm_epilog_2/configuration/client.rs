// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use serde_aux::field_attributes::deserialize_number_from_string;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Settings {
    #[serde(default = "default_addr")]
    pub addr: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Settings {
    pub fn get_addr(&self) -> String {
        format!("{}:{}", self.addr, self.port)
    }
}

fn default_addr() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    4687
}

/// Loads the configuration from a file `configuration.{yaml,json,toml,...}`
#[tracing::instrument(name = "Loading configuration")]
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let settings = config::Config::builder();

    let settings = match std::env::args().nth(1) {
        Some(path) => settings.add_source(
            config::File::from(path.as_ref())
                .required(false)
                .format(config::FileFormat::Yaml),
        ),
        None => settings,
    };

    let settings = settings.add_source(
        config::Environment::with_prefix("AUDITOR_SLURM_COLLECTOR_CLIENT")
            .separator("__")
            .prefix_separator("_"),
    );

    settings.build()?.try_deserialize()
}
