use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use chrono::{DateTime, Local, TimeDelta};
use serde::Deserialize;
use tracing_subscriber::filter::LevelFilter;

pub fn load_configuration(p: impl AsRef<Path>) -> Config {
    let yaml = fs::read_to_string(p.as_ref()).expect("Cannot open config file");
    let config: DeConfig = serde_yaml::from_str(&yaml).expect("Config is not valid yaml");
    config.into()
}

#[derive(Deserialize)]
#[serde(from = "Config")]
struct DeConfig(Config);

impl From<Config> for DeConfig {
    fn from(mut value: Config) -> Self {
        for status in value.job_filter.status.iter_mut() {
            status.make_ascii_lowercase()
        }
        Self(value)
    }
}

impl From<DeConfig> for Config {
    fn from(value: DeConfig) -> Self {
        value.0
    }
}

//#[serde_with::serde_as]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub auditor_addr: String,
    #[serde(default = "default_auditor_port")]
    pub auditor_port: u16,
    pub prometheus_addr: String,
    pub prometheus_port: u16,
    #[serde(default = "default_record_prefix")]
    pub record_prefix: String,
    #[serde(default = "default_earliest_datetime")]
    pub earliest_datetime: DateTime<Local>,
    #[serde(default = "default_auditor_timeout")]
    #[serde(deserialize_with = "deserialize_timedelta")]
    pub auditor_timeout: TimeDelta,
    #[serde(default = "default_prometheus_timeout")]
    #[serde(deserialize_with = "deserialize_timedelta")]
    pub prometheus_timeout: TimeDelta,
    #[serde(default = "default_collect_interval")]
    #[serde(deserialize_with = "deserialize_timedelta")]
    pub collect_interval: TimeDelta,
    #[serde(default = "default_send_interval")]
    #[serde(deserialize_with = "deserialize_timedelta")]
    pub send_interval: TimeDelta,
    #[serde(default = "default_database_path")]
    pub database_path: PathBuf,
    #[serde(default)]
    pub job_filter: JobFilterSettings,
    //#[serde(default)] // bool defaults to false
    //pub delete_jobs: bool,
    #[serde(default = "default_backlog_interval")]
    #[serde(deserialize_with = "deserialize_duration")]
    pub backlog_interval: Duration,
    #[serde(default = "default_backlog_maxtries")]
    pub backlog_maxretries: u16,
    #[serde(default = "default_log_level")]
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: LevelFilter,
}

fn default_auditor_port() -> u16 {
    8000
}
fn default_record_prefix() -> String {
    //"KUBE".to_owned()
    "".to_owned()
}
fn default_earliest_datetime() -> DateTime<Local> {
    Local::now()
}
fn default_auditor_timeout() -> TimeDelta {
    TimeDelta::try_seconds(10).unwrap()
}
fn default_prometheus_timeout() -> TimeDelta {
    TimeDelta::try_seconds(60).unwrap()
}
fn default_collect_interval() -> TimeDelta {
    TimeDelta::try_seconds(60).unwrap()
}
fn default_send_interval() -> TimeDelta {
    TimeDelta::try_seconds(60).unwrap()
}
fn default_database_path() -> PathBuf {
    PathBuf::from(".")
}
fn default_backlog_interval() -> Duration {
    Duration::from_secs(300)
}
fn default_backlog_maxtries() -> u16 {
    2
}
fn default_log_level() -> LevelFilter {
    LevelFilter::INFO
}

pub fn deserialize_log_level<'de, D>(deserializer: D) -> Result<LevelFilter, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    LevelFilter::from_str(&s.to_lowercase()).map_err(serde::de::Error::custom)
}

pub fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let seconds = i64::deserialize(deserializer)?;
    if seconds < 1 {
        Err(serde::de::Error::custom(
            "durations should be greater than zero",
        ))
    } else {
        Ok(Duration::from_secs(seconds as u64))
    }
}

pub fn deserialize_timedelta<'de, D>(deserializer: D) -> Result<TimeDelta, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let seconds = i64::deserialize(deserializer)?;
    if seconds < 1 {
        Err(serde::de::Error::custom(
            "durations should be greater than zero",
        ))
    } else {
        let dur = TimeDelta::try_seconds(seconds).ok_or(serde::de::Error::custom(format!(
            "Cannot convert {} seconds to TimeDelta",
            seconds
        )))?;
        if let Err(e) = dur.to_std() {
            Err(serde::de::Error::custom(e))
        } else {
            Ok(dur)
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct JobFilterSettings {
    /// Potentially interesting: complete, failed, suspended
    #[serde(default = "default_job_filter_status")]
    pub status: Vec<String>,
    #[serde(default = "default_job_filter_namespace")]
    pub namespace: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
}

impl Default for JobFilterSettings {
    fn default() -> Self {
        Self {
            status: default_job_filter_status(),
            namespace: default_job_filter_namespace(),
            labels: Vec::with_capacity(0),
        }
    }
}

fn default_job_filter_status() -> Vec<String> {
    vec!["completed".into()]
}

fn default_job_filter_namespace() -> Vec<String> {
    vec!["default".into()]
}
