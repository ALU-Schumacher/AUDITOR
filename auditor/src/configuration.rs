// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::telemetry::deserialize_log_level;
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::ConnectOptions;
use tracing_subscriber::filter::LevelFilter;

#[derive(serde::Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: AuditorSettings,
    #[serde(default = "default_metrics")]
    pub metrics: MetricsSettings,
    #[serde(default = "default_log_level")]
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: LevelFilter,
}

fn default_log_level() -> LevelFilter {
    LevelFilter::INFO
}

#[derive(serde::Deserialize, Debug)]
pub struct AuditorSettings {
    #[serde(default = "default_addr")]
    pub addr: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

fn default_addr() -> String {
    "127.0.0.1".to_string()
}

#[derive(serde::Deserialize, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

#[derive(serde::Deserialize, Debug)]
pub struct MetricsSettings {
    pub database: DatabaseMetricsSettings,
}

#[serde_with::serde_as]
#[derive(serde::Deserialize, Debug)]
pub struct DatabaseMetricsSettings {
    #[serde(default = "default_db_metrics_frequency")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub frequency: chrono::Duration,
    pub metrics: Vec<crate::metrics::DatabaseMetricsOptions>,
}

fn default_db_metrics_frequency() -> chrono::Duration {
    chrono::Duration::try_seconds(30).expect("This should never fail")
}

fn default_metrics() -> MetricsSettings {
    MetricsSettings {
        database: DatabaseMetricsSettings {
            frequency: default_db_metrics_frequency(),
            metrics: vec![],
        },
    }
}

impl DatabaseSettings {
    /// Returns the connection options for the PostgreSQL database without database name
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    /// Returns the connection options for the PostgreSQL database with database name
    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db()
            .database(&self.database_name)
            .log_statements(tracing::log::LevelFilter::Trace)
    }
}

/// Loads the configuration from a file `configuration.{yaml,json,toml,...}`
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration");

    // if the directory doesn't exist, we're probably in the wrong directory. Let's get inside
    // "auditor/configuration" then!
    let configuration_directory = if configuration_directory.exists() {
        configuration_directory
    } else {
        base_path.join("auditor").join("configuration")
    };

    let environment: Environment = std::env::var("AUDITOR_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse AUDITOR_ENVIRONMENT.");

    let settings = config::Config::builder()
        .add_source(config::File::from(configuration_directory.join("base")).required(false))
        .add_source(
            config::File::from(configuration_directory.join(environment.as_str())).required(false),
        );

    let settings = match std::env::args().nth(1) {
        Some(file) => settings.add_source(
            config::File::from(file.as_ref())
                .required(false)
                .format(config::FileFormat::Yaml),
        ),
        None => settings,
    };

    let settings = settings.add_source(
        config::Environment::with_prefix("AUDITOR")
            .separator("__")
            .prefix_separator("_"),
    );

    settings.build()?.try_deserialize()
}

// The possible runtime environment for AUDITOR.
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{other} is not a supported environment. Use either `local` or `production`."
            )),
        }
    }
}
