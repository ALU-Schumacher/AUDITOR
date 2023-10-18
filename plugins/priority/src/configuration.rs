// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use auditor::telemetry::deserialize_log_level;
use chrono::Duration;
use serde_aux::field_attributes::deserialize_number_from_string;
use std::collections::HashMap;
use tracing_subscriber::filter::LevelFilter;

#[derive(serde::Deserialize, Debug, Clone)]
pub enum ComputationMode {
    FullSpread,
    ScaledBySum,
}

#[serde_with::serde_as]
#[derive(serde::Deserialize, Debug, Clone)]
pub struct Settings {
    pub auditor: AuditorSettings,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(default = "default_timeout")]
    pub timeout: i64,
    pub components: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(default = "default_min_priority")]
    pub min_priority: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(default = "default_max_priority")]
    pub max_priority: u64,
    pub group_mapping: HashMap<String, Vec<String>>,
    #[serde(default = "default_command")]
    pub commands: Vec<String>,
    #[serde_as(as = "Option<serde_with::DurationSeconds<i64>>")]
    pub duration: Option<Duration>,
    #[serde(default = "default_computation_mode")]
    pub computation_mode: ComputationMode,
    #[serde(default = "default_prometheus_frequency")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub frequency: chrono::Duration,
    #[serde(default = "default_log_level")]
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: LevelFilter,
    pub prometheus: Option<PrometheusSettings>,
}

#[serde_with::serde_as]
#[derive(serde::Deserialize, Debug, Clone)]
pub struct AuditorSettings {
    #[serde(default = "default_addr")]
    pub addr: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(default = "default_port")]
    pub port: u16,
}

#[serde_with::serde_as]
#[derive(serde::Deserialize, Debug, Clone)]
pub struct PrometheusSettings {
    #[serde(default = "default_enable_option")]
    pub enable: bool,
    #[serde(default = "default_prometheus_addr")]
    pub addr: String,
    #[serde(default = "default_prometheus_port")]
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub metrics: Vec<PrometheusMetricsOptions>,
}

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrometheusMetricsOptions {
    ResourceUsage,
    Priority,
}

fn default_enable_option() -> bool {
    true
}

fn default_prometheus_addr() -> String {
    "0.0.0.0".to_string()
}

fn default_prometheus_port() -> u16 {
    9090
}

fn default_prometheus_frequency() -> chrono::Duration {
    chrono::Duration::seconds(3600)
}

fn default_log_level() -> LevelFilter {
    LevelFilter::INFO
}

fn default_addr() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8000
}

fn default_timeout() -> i64 {
    30
}

fn default_min_priority() -> u64 {
    0
}

fn default_max_priority() -> u64 {
    1024
}

fn default_command() -> Vec<String> {
    vec!["/usr/bin/scontrol update PartitionName={1} PriorityJobFactor={priority}".to_string()]
}

fn default_computation_mode() -> ComputationMode {
    ComputationMode::ScaledBySum
}

/// Loads the configuration from a file `configuration.{yaml,json,toml,...}`
#[tracing::instrument(name = "Loading configuration")]
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration").join("priority-plugin");

    let settings = config::Config::builder()
        .add_source(config::File::from(configuration_directory.join("base")).required(false));
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
