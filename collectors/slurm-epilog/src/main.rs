// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anyhow::Error;
use auditor::domain::{Component, RecordAdd, Score};
use auditor::telemetry::{get_subscriber, init_subscriber};
use auditor_client::AuditorClientBuilder;
use chrono::{DateTime, Local, NaiveDateTime, Utc, offset::FixedOffset};
use regex::Regex;
use std::collections::HashMap;
use std::env;
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
    let local_offset = Local::now().offset().local_minus_utc();
    Ok(DateTime::<Utc>::from(
        NaiveDateTime::parse_from_str(timestamp.as_ref(), "%Y-%m-%dT%H:%M:%S")?
            .and_local_timezone(FixedOffset::east_opt(local_offset).unwrap())
            .unwrap(),
    ))
}

#[tracing::instrument(
    name = "Construct components from job info and configuration",
    level = "debug"
)]
fn construct_components(config: &configuration::Settings, job: &Job) -> Vec<Component> {
    config
        .components
        .iter()
        .filter(|c| {
            c.only_if.is_none() || {
                let only_if = c.only_if.as_ref().unwrap();
                let re = Regex::new(&only_if.matches)
                    .unwrap_or_else(|_| panic!("Invalid regex expression: {}", &only_if.matches));
                re.is_match(&job[&only_if.key])
            }
        })
        .cloned()
        .map(|c| {
            Component::new(
                c.name,
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
                        Score::new(s.name.clone(), s.value)
                            .unwrap_or_else(|_| panic!("Cannot construct score from {s:?}"))
                    })
                    .collect(),
            )
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = configuration::get_configuration()?;
    // Set up logging
    let subscriber = get_subscriber(
        "AUDITOR-slurm-epilog-collector".into(),
        config.log_level,
        std::io::stdout,
    );
    init_subscriber(subscriber);

    let run_id = Uuid::new_v4();
    let span = tracing::info_span!(
        "Running slurm epilog collector",
        %run_id,
    );
    let _span_guard = span.enter();

    debug!(?config, "Loaded config");

    let client = if config.tls_config.use_tls {
        let tls_config = &config.tls_config;

        let _ = tls_config
            .validate_tls_paths()
            .map_err(|e| format!("Configuration error: {e}"));

        let ca_cert_path = tls_config.ca_cert_path.as_ref().unwrap();
        let client_key_path = tls_config.client_key_path.as_ref().unwrap();
        let client_cert_path = tls_config.client_cert_path.as_ref().unwrap();

        // Build client with TLS
        AuditorClientBuilder::new()
            .address(&config.addr, config.port)
            .with_tls(client_cert_path, client_key_path, ca_cert_path)
            .build()?
    } else {
        // Build client without TLS
        AuditorClientBuilder::new()
            .address(&config.addr, config.port)
            .build()?
    };

    let job_id = get_slurm_job_id().expect("Collector not run in the context of a Slurm epilog");

    info!(slurm_job_id = job_id, "Acquired SLURM job ID");

    let job = get_slurm_job_info(job_id)?;

    debug!(?job, "Acquired SLURM job info");

    let record = RecordAdd::new(
        format!("{}-{job_id}", &config.record_prefix),
        HashMap::from([
            ("site_id".to_string(), vec![config.site_id.clone()]),
            (
                "user_id".to_string(),
                vec![job["UserId"].split('(').take(1).collect::<Vec<_>>()[0].to_string()],
            ),
            (
                "group_id".to_string(),
                vec![job["GroupId"].split('(').take(1).collect::<Vec<_>>()[0].to_string()],
            ),
        ]),
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
