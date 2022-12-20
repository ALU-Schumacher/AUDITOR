// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// use std::time::Duration;

use chrono::{offset::FixedOffset, DateTime, Duration, Local, NaiveDateTime, TimeZone, Utc};
use color_eyre::eyre::{eyre, Report, Result, WrapErr};
use itertools::Itertools;
use once_cell::unsync::Lazy;
use regex::{Captures, Regex, RegexSet};
use serde_aux::field_attributes::deserialize_number_from_string;

#[serde_with::serde_as]
#[derive(serde::Deserialize, Debug, Clone)]
pub struct Settings {
    #[serde(default = "default_collector_addr")]
    pub collector_addr: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(default = "default_collector_port")]
    pub collector_port: u16,
    #[serde(default = "default_addr")]
    pub addr: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_record_prefix")]
    pub record_prefix: String,
    #[serde(default = "default_sites")]
    pub sites: Vec<SiteConfig>,
    #[serde(default = "default_earliest_datetime")]
    pub earliest_datetime: DateTime<Local>,
    #[serde(default = "default_components")]
    pub components: Vec<ComponentConfig>,
    #[serde(default = "default_sacct_frequency")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub sacct_frequency: Duration,
    #[serde(default = "default_sender_frequency")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub sender_frequency: Duration,
    #[serde(default = "default_database_path")]
    pub database_path: String,
    /// Potentially interesting: completed, failed, node_fail
    #[serde(default = "default_job_status")]
    pub job_status: Vec<String>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct SiteConfig {
    pub name: String,
    pub only_if: Option<OnlyIf>,
}

impl SiteConfig {
    fn keys(&self) -> Option<(String, ParsableType)> {
        self.only_if.as_ref().map(|oif| oif.key())
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ComponentConfig {
    pub name: String,
    pub key: String,
    #[serde(default = "default_key_type")]
    pub key_type: ParsableType,
    #[serde(default = "default_score")]
    pub scores: Vec<ScoreConfig>,
    pub only_if: Option<OnlyIf>,
}

impl ComponentConfig {
    fn keys(&self) -> Vec<(String, ParsableType)> {
        let mut keys: Vec<(String, ParsableType)> =
            self.scores.iter().flat_map(|s| s.keys()).collect();
        keys.push((self.key.clone(), self.key_type));
        if let Some(ref oif) = self.only_if {
            keys.push(oif.key());
        }
        keys
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ScoreConfig {
    pub name: String,
    pub value: f64,
    pub only_if: Option<OnlyIf>,
}

impl ScoreConfig {
    fn keys(&self) -> Vec<(String, ParsableType)> {
        self.only_if.iter().map(|oif| oif.key()).collect()
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct OnlyIf {
    pub key: String,
    pub matches: String,
}

impl OnlyIf {
    fn key(&self) -> (String, ParsableType) {
        (self.key.clone(), ParsableType::String)
    }
}

fn default_addr() -> String {
    "127.0.0.1".to_string()
}

fn default_collector_addr() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8000
}

fn default_collector_port() -> u16 {
    4687
}

fn default_record_prefix() -> String {
    "slurm".to_string()
}

fn default_score() -> Vec<ScoreConfig> {
    vec![]
}

fn default_sites() -> Vec<SiteConfig> {
    vec![SiteConfig {
        name: "NOT_CONFIGURED".to_string(),
        only_if: None,
    }]
}

fn default_key_type() -> ParsableType {
    ParsableType::default()
}

fn default_earliest_datetime() -> DateTime<Local> {
    Local.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap()
}

fn default_sacct_frequency() -> Duration {
    Duration::seconds(10)
}

fn default_sender_frequency() -> Duration {
    Duration::seconds(1)
}

fn default_database_path() -> String {
    "sqlite://testdb.db".into()
}

fn default_job_status() -> Vec<String> {
    vec!["completed".into()]
}

fn default_components() -> Vec<ComponentConfig> {
    vec![ComponentConfig {
        name: "Cores".into(),
        key: "NCPUS".into(),
        key_type: ParsableType::default(),
        scores: vec![],
        only_if: None,
    }]
}

impl Settings {
    pub fn get_keys(&self) -> Vec<(String, ParsableType)> {
        let mut keys = self.sites.iter().flat_map(|s| s.keys()).collect::<Vec<_>>();
        keys.extend(self.components.iter().flat_map(|c| c.keys()));
        keys.into_iter().unique_by(|t| t.0.clone()).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AllowedTypes {
    String(String),
    Integer(i64),
    DateTime(DateTime<Utc>),
}

impl AllowedTypes {
    pub fn extract_string(&self) -> Result<String, Report> {
        if let AllowedTypes::String(string) = self {
            Ok(string.clone())
        } else {
            Err(eyre!("Cannot extract string!"))
        }
    }
    pub fn extract_i64(&self) -> Result<i64, Report> {
        if let AllowedTypes::Integer(integer) = *self {
            Ok(integer)
        } else {
            Err(eyre!("Cannot extract integer!"))
        }
    }
    pub fn extract_datetime(&self) -> Result<DateTime<Utc>, Report> {
        if let AllowedTypes::DateTime(datetime) = *self {
            Ok(datetime)
        } else {
            Err(eyre!("Cannot extract datetime!"))
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ParsableType {
    #[default]
    Integer,
    IntegerMega,
    Time,
    String,
    DateTime,
    Id,
}

impl ParsableType {
    pub fn parse<T: AsRef<str>>(&self, input: T) -> Result<AllowedTypes, Report> {
        let input = input.as_ref();
        Ok(match self {
            ParsableType::Integer => AllowedTypes::Integer(
                input
                    .parse()
                    .map_err(|e| {
                        tracing::error!("Cannot parse {input} into i64.");
                        e
                    })
                    .context(format!("Parsing of {input} into i64 failed."))?,
            ),
            ParsableType::IntegerMega => {
                let mut chars = input.chars();
                chars.next_back();
                let input = chars.as_str();
                AllowedTypes::Integer(
                    input
                        .parse()
                        .map_err(|e| {
                            tracing::error!("Cannot parse {input} into i64.");
                            e
                        })
                        .context(format!("Parsing of {input} into i64 failed."))?,
                )
            }
            ParsableType::Time => {
                let set = Lazy::new(|| {
                    RegexSet::new([
                        r"(?P<min>\d{2}):(?P<sec>\d{2})\.(?P<milli>\d+)",
                        r"(?P<hour>\d{2}):(?P<min>\d{2}):(?P<sec>\d{2})",
                    ])
                    .unwrap()
                });
                let regexes = Lazy::new(|| {
                    set.patterns()
                        .iter()
                        .map(|pat| Regex::new(pat).unwrap())
                        .collect::<Vec<_>>()
                });
                if !set.is_match(input) {
                    return Err(eyre!("Cannot parse time string: {}", input));
                }

                let captures: Vec<Captures> = set
                    .matches(input)
                    .into_iter()
                    .map(|match_idx| &regexes[match_idx])
                    .map(|pat| -> Result<Captures> {
                        pat.captures(input).ok_or_else(|| {
                            eyre!(
                                "Impossible error when parsing time string: {}. Tell Stefan!",
                                input
                            )
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;

                if captures.is_empty() {
                    return Err(eyre!(
                        "No regex pattern matched when parsing time {}. This is impossible.",
                        input
                    ));
                }

                if captures.len() > 1 {
                    tracing::warn!("Multiple regex patterns matched when parsing time {}. This should not happen. Taking first one.", input);
                }

                // Unwrap is fine because we have ensured that there is exactly one element.
                let cap = captures.into_iter().next().unwrap();

                let pm = |name: &'static str, reg_match: &Captures| -> Result<i64> {
                    Ok(if let Some(a) = reg_match.name(name) {
                        a.as_str().parse::<i64>().wrap_err_with(|| {
                            format!(
                                "Failed parsing {} match group ({}) to i64",
                                name,
                                a.as_str()
                            )
                        })?
                    } else {
                        0
                    })
                };

                AllowedTypes::Integer(
                    pm("milli", &cap)?
                        + pm("sec", &cap)? * 1000
                        + pm("min", &cap)? * 60_000
                        + pm("hour", &cap)? * 1_440_000,
                )
            }
            ParsableType::String => AllowedTypes::String(input.to_owned()),
            ParsableType::DateTime => {
                let local_offset = Local::now().offset().local_minus_utc();
                AllowedTypes::DateTime(DateTime::<Utc>::from(DateTime::<Local>::from_local(
                    NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S")?,
                    FixedOffset::east_opt(local_offset).unwrap(),
                )))
            }
            ParsableType::Id => {
                AllowedTypes::String(input.split('(').take(1).collect::<Vec<_>>()[0].to_owned())
            }
        })
    }
}

/// Loads the configuration from a file `configuration.{yaml,json,toml,...}`
#[tracing::instrument(name = "Loading configuration")]
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration").join("slurm-collector");

    let settings = config::Config::builder()
        .add_source(config::File::from(configuration_directory.join("base")).required(false));
    let settings = match std::env::args().nth(1) {
        Some(file) => settings.add_source(
            config::File::from(file.as_ref())
                .required(true)
                .format(config::FileFormat::Yaml),
        ),
        None => settings,
    };

    // Should only be used for (temporarily) overwriting some configurations like addr or port.
    // This is definitely not meant to do the full configuration with.
    let settings = settings.add_source(
        config::Environment::with_prefix("AUDITOR_SLURM_COLLECTOR")
            .separator("__")
            .prefix_separator("_"),
    );

    settings.build()?.try_deserialize()
}
