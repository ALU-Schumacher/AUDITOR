// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anyhow::{anyhow, Error};
use chrono::{offset::FixedOffset, DateTime, Local, NaiveDateTime, Utc};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use serde_aux::field_attributes::deserialize_number_from_string;

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
    #[serde(default = "default_string")]
    pub site_id: String,
    #[serde(default = "default_components")]
    pub components: Vec<ComponentConfig>,
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
    pub factor: f64,
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

fn default_string() -> String {
    "none".to_string()
}

fn default_score() -> Vec<ScoreConfig> {
    vec![]
}

fn default_key_type() -> ParsableType {
    ParsableType::default()
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
        self.components
            .iter()
            .flat_map(|c| c.keys())
            .unique_by(|t| t.0.clone())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AllowedTypes {
    String(String),
    Integer(i64),
    DateTime(DateTime<Utc>),
}

impl AllowedTypes {
    pub fn extract_string(&self) -> Result<String, Error> {
        if let AllowedTypes::String(string) = self {
            Ok(string.clone())
        } else {
            Err(anyhow!("Cannot extract string!"))
        }
    }

    pub fn extract_i64(&self) -> Result<i64, Error> {
        if let AllowedTypes::Integer(integer) = *self {
            Ok(integer)
        } else {
            Err(anyhow!("Cannot extract integer!"))
        }
    }

    pub fn extract_datetime(&self) -> Result<DateTime<Utc>, Error> {
        if let AllowedTypes::DateTime(datetime) = *self {
            Ok(datetime)
        } else {
            Err(anyhow!("Cannot extract datetime!"))
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ParsableType {
    #[default]
    Integer,
    Time,
    String,
    DateTime,
    Id,
}

impl ParsableType {
    pub fn parse<T: AsRef<str>>(&self, input: T) -> Result<AllowedTypes, Error> {
        let input = input.as_ref();
        Ok(match self {
            ParsableType::Integer => AllowedTypes::Integer(
                input
                    .parse()
                    .unwrap_or_else(|_| panic!("Cannot parse {} into i64.", input)),
            ),
            ParsableType::Time => {
                lazy_static! {
                    static ref RE: Regex = Regex::new(r"(\d{2}):(\d{2}).(\d+)").unwrap();
                }
                let cap = RE.captures(input).unwrap_or_else(|| {
                    panic!(
                        "Cannot parse duration {}. Duration must have the format MM:SS.MILLI.",
                        input
                    )
                });
                let cap = cap
                    .iter()
                    .map(|c| {
                        c.unwrap()
                            .as_str()
                            .parse::<i64>()
                            .unwrap_or_else(|_| panic!("Cannot parse {} into i64.", input))
                    })
                    .collect::<Vec<_>>();
                let (min, sec, milli): (i64, i64, i64) = (cap[0], cap[1], cap[2]);
                AllowedTypes::Integer(milli + sec * 1000 + min * 60_000)
            }
            ParsableType::String => AllowedTypes::String(input.to_owned()),
            ParsableType::DateTime => {
                let local_offset = Local::now().offset().local_minus_utc();
                AllowedTypes::DateTime(DateTime::<Utc>::from(DateTime::<Local>::from_local(
                    NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S")?,
                    FixedOffset::east(local_offset),
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
    let settings = config::Config::builder();
    let settings = match std::env::args().nth(1) {
        Some(file) => settings.add_source(
            config::File::from(file.as_ref())
                .required(false)
                .format(config::FileFormat::Yaml),
        ),
        None => settings,
    };
    let settings = settings.add_source(
        config::Environment::with_prefix("AUDITOR_SLURM_COLLECTOR")
            .separator("__")
            .prefix_separator("_"),
    );

    settings.build()?.try_deserialize()
}
