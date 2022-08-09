// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anyhow::Error;
use auditor::client::AuditorClient;
use auditor::constants::FORBIDDEN_CHARACTERS;
use auditor::domain::{Component, RecordAdd, Score};
use auditor::telemetry::{get_subscriber, init_subscriber};
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::process::Command;
use tracing::{debug, info};
use uuid::Uuid;

mod configuration;

#[tracing::instrument(name = "Obtaining Slurm job id from environment")]
fn get_slurm_job_id() -> Result<u64, Error> {
    Ok(env::var("SLURM_JOB_ID")?.parse()?)
}

type Job = HashMap<String, String>;

#[tracing::instrument(name = "Getting Slurm job info via scontrol")]
fn get_slurm_job_info(job_id: u64) -> Result<Job, Error> {
    Ok(std::str::from_utf8(
        &Command::new("/usr/bin/scontrol")
            .arg("show")
            .arg("job")
            .arg(job_id.to_string())
            .arg("--details")
            .output()?
            .stdout,
    )?
    .split_whitespace()
    .filter_map(|s| {
        if let Some((k, v)) = s.split_once('=') {
            Some((k.to_string(), v.to_string()))
        } else {
            None
        }
    })
    .collect())
}

#[tracing::instrument(name = "Parsing Slurm timestamp", level = "debug")]
fn parse_slurm_timestamp<T: AsRef<str> + std::fmt::Debug>(
    timestamp: T,
) -> Result<DateTime<Utc>, Error> {
    Ok(DateTime::<Utc>::from_utc(
        NaiveDateTime::parse_from_str(timestamp.as_ref(), "%Y-%m-%dT%H:%M:%S")?,
        Utc,
    ))
}

#[tracing::instrument(name = "Remove forbidden characters from string", level = "debug")]
fn make_string_valid<T: AsRef<str> + fmt::Debug>(input: T) -> String {
    input.as_ref().replace(&FORBIDDEN_CHARACTERS[..], "")
}

#[tracing::instrument(
    name = "Construct components from job info and configuration",
    level = "debug"
)]
fn construct_components(config: &configuration::Settings, job: &Job) -> Vec<Component> {
    config
        .components
        .iter()
        .cloned()
        .filter(|c| {
            c.only_if.is_none() || {
                let only_if = c.only_if.as_ref().unwrap();
                let re = Regex::new(&only_if.matches)
                    .unwrap_or_else(|_| panic!("Invalid regex expression: {}", &only_if.matches));
                re.is_match(&job[&only_if.key])
            }
        })
        .map(|c| {
            Component::new(
                make_string_valid(c.name),
                job[&c.key].parse().unwrap_or_else(|_| {
                    panic!(
                        "Cannot parse key {} (value: {}) into u64.",
                        c.key, job[&c.key]
                    )
                }),
            )
            .expect("Cannot construct component. Please check your configuration!")
            .with_scores(
                c.scores
                    .iter()
                    .filter(|s| {
                        s.only_if.is_none() || {
                            let only_if = s.only_if.as_ref().unwrap();
                            let re = Regex::new(&only_if.matches).unwrap_or_else(|_| {
                                panic!("Invalid regex expression: {}", &only_if.matches)
                            });
                            re.is_match(&job[&only_if.key])
                        }
                    })
                    .map(|s| {
                        Score::new(s.name.clone(), s.factor)
                            .unwrap_or_else(|_| panic!("Cannot construct score from {:?}", s))
                    })
                    .collect(),
            )
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Set up logging
    let subscriber = get_subscriber(
        "AUDITOR-slurm-epilog-collector".into(),
        "info".into(),
        std::io::stdout,
    );
    init_subscriber(subscriber);

    let run_id = Uuid::new_v4();
    let span = tracing::info_span!(
        "Running slurm epilog collector",
        %run_id,
    );
    let _span_guard = span.enter();

    let config = configuration::get_configuration()?;

    debug!(?config, "Loaded config");

    let client = AuditorClient::new(&config.addr, config.port)?;

    let job_id = get_slurm_job_id().expect("Collector not run in the context of a Slurm epilog");

    info!(slurm_job_id = job_id, "Acquired SLURM job ID");

    let job = get_slurm_job_info(job_id)?;

    debug!(?job, "Acquired SLURM job info");

    let record = RecordAdd::new(
        format!("{}-{}", make_string_valid(&config.record_prefix), job_id),
        make_string_valid(&config.site_id),
        make_string_valid(&job["UserId"].split('(').take(1).collect::<Vec<_>>()[0]),
        make_string_valid(&job["GroupId"].split('(').take(1).collect::<Vec<_>>()[0]),
        construct_components(&config, &job),
        parse_slurm_timestamp(&job["StartTime"])?,
    )
    .expect("Could not construct record")
    .with_stop_time(parse_slurm_timestamp(&job["EndTime"])?);

    debug!(?record, "Constructed record.");

    info!("Sending record to AUDITOR instance.");
    client.add(&record).await?;

    Ok(())
}
