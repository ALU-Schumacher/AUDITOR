// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::telemetry::deserialize_log_level;
use rustls::ServerConfig;
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::ConnectOptions;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::collections::HashMap;
use tracing_subscriber::filter::LevelFilter;

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default)]
    pub environment: Environment,
    pub database: DatabaseSettings,
    pub application: AuditorSettings,
    #[serde(default = "default_metrics")]
    pub metrics: MetricsSettings,
    #[serde(default = "default_log_level")]
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: LevelFilter,
    pub tls_config: Option<TLSConfig>,
    pub rbac_config: Option<RbacConfig>,
    #[serde(default = "default_ignore_record_exists_error")]
    pub ignore_record_exists_error: bool,
    pub archival_config: Option<ArchivalConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum CompressionType {
    Gzip,
    Snappy,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ArchivalConfig {
    pub archive_older_than_months: i32,
    pub archive_path: String,
    #[serde(default = "default_archive_file_prefix")]
    pub archive_file_prefix: String,
    pub cron_schedule: String, // e.g., "0 0 2 1 * *" // Monthly
    #[serde(default = "default_compression_type")]
    pub compression_type: CompressionType,
}

fn default_compression_type() -> CompressionType {
    CompressionType::Gzip
}

fn default_archive_file_prefix() -> String {
    "auditor".to_string()
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RbacConfig {
    #[serde(default = "default_enforce_rbac")]
    pub enforce_rbac: bool,
    #[serde(default = "default_base_policies")]
    pub base_policies: Vec<Vec<String>>,
    pub monitoring_role_cn: Option<Vec<String>>,
    pub write_access_cn: Option<Vec<String>>,
    pub read_access_cn: Option<Vec<String>>,
    pub data_access_rules: Option<Vec<Cn>>,
}

fn default_ignore_record_exists_error() -> bool {
    false
}

fn default_enforce_rbac() -> bool {
    false
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Cn {
    pub reader_cn: String,
    pub meta_info: HashMap<String, Vec<String>>,
}

fn default_base_policies() -> Vec<Vec<String>> {
    vec![
        vec![
            "monitoring_role".to_string(),
            "/metrics".to_string(),
            "GET".to_string(),
        ],
        vec![
            "write_access_base".to_string(),
            "/record".to_string(),
            "POST".to_string(),
        ],
        vec![
            "write_access_base".to_string(),
            "/record".to_string(),
            "PUT".to_string(),
        ],
        vec![
            "write_access_base".to_string(),
            "/records".to_string(),
            "POST".to_string(),
        ],
        vec![
            "write_access_base".to_string(),
            "/healthcheck".to_string(),
            "GET".to_string(),
        ],
        vec![
            "read_access_base".to_string(),
            "/records".to_string(),
            "GET".to_string(),
        ],
        vec![
            "read_access_base".to_string(),
            "/record/*".to_string(),
            "GET".to_string(),
        ],
        vec![
            "read_access_base".to_string(),
            "/healthcheck".to_string(),
            "GET".to_string(),
        ],
    ]
}

//Set the default values for TLSConfig options
#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct TLSConfig {
    pub use_tls: bool,
    pub https_addr: Option<Vec<String>>,
    #[serde(default = "default_https_port")]
    pub https_port: u16,
    pub ca_cert_path: Option<String>,
    pub server_cert_path: Option<String>,
    pub server_key_path: Option<String>,
}

fn default_https_port() -> u16 {
    8443u16
}

impl TLSConfig {
    /// Checks if TLS is enabled and required paths are provided.
    pub fn validate_tls_paths(&self) -> Result<(), &'static str> {
        if self.use_tls {
            if self.ca_cert_path.is_none() {
                return Err("ca_cert_path is required when use_tls is true");
            }
            if self.server_cert_path.is_none() {
                return Err("server_cert_path is required when use_tls is true");
            }
            if self.server_key_path.is_none() {
                return Err("server_key_path is required when use_tls is true");
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct TLSParams {
    pub config: ServerConfig,
    pub https_addr: Option<Vec<String>>,
    pub https_port: u16,
    pub use_tls: bool,
}

fn default_log_level() -> LevelFilter {
    LevelFilter::INFO
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct AuditorSettings {
    #[serde(default = "default_addr")]
    pub addr: Vec<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    #[serde(default = "default_workers")]
    pub web_workers: usize,
}

fn default_addr() -> Vec<String> {
    vec!["127.0.0.1".to_string()]
}

fn default_workers() -> usize {
    if let Ok(num) = std::thread::available_parallelism() {
        std::cmp::min(num.get(), 4)
    } else {
        tracing::warn!("Cannot determine how many web workers to use. Fall back to 2.");
        2
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct MetricsSettings {
    pub database: DatabaseMetricsSettings,
}

#[serde_with::serde_as]
#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct DatabaseMetricsSettings {
    #[serde(default = "default_db_metrics_frequency")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub frequency: chrono::Duration,
    pub metrics: Vec<crate::metrics::DatabaseMetricsOptions>,
    #[serde(default = "default_meta_key_site")]
    pub meta_key_site: String,
    #[serde(default = "default_meta_key_group")]
    pub meta_key_group: String,
    #[serde(default = "default_meta_key_user")]
    pub meta_key_user: String,
}

fn default_meta_key_site() -> String {
    "site".to_string()
}

fn default_meta_key_group() -> String {
    "group".to_string()
}

fn default_meta_key_user() -> String {
    "user".to_string()
}

fn default_db_metrics_frequency() -> chrono::Duration {
    chrono::Duration::try_seconds(30).expect("This should never fail")
}

fn default_metrics() -> MetricsSettings {
    MetricsSettings {
        database: DatabaseMetricsSettings {
            frequency: default_db_metrics_frequency(),
            metrics: vec![],
            meta_key_site: default_meta_key_site(),
            meta_key_group: default_meta_key_group(),
            meta_key_user: default_meta_key_user(),
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
        .ok()
        .map(Environment::try_from)
        .transpose()
        .map_err(config::ConfigError::Message)?
        .unwrap_or_default();

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
            .prefix_separator("_")
            .list_separator(",")
            .with_list_parse_key("application.addr")
            .with_list_parse_key("rbac_config.read_access_cn")
            .with_list_parse_key("rbac_config.write_access_cn")
            .try_parsing(true),
    );

    settings.build()?.try_deserialize()
}

// The possible runtime environment for AUDITOR.
#[derive(serde::Deserialize, Debug, Default)]
#[serde(try_from = "String")]
pub enum Environment {
    #[default]
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
