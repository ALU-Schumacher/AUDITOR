// use crate::slurm_epilog::configuration::get_configuration;
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

mod configuration;

#[tracing::instrument(name = "Obtaining Slurm job id from environment")]
fn get_slurm_job_id() -> Result<u64, Error> {
    Ok(env::var("SLURM_JOB_ID")?.parse()?)
}

type Job = HashMap<String, String>;

#[tracing::instrument(name = "Getting Slurm job info via scontrol")]
fn get_slurm_job_info(job_id: u64) -> Result<Job, Error> {
    Ok(std::str::from_utf8(
        &Command::new("scontrol")
            .arg("show")
            .arg("job")
            .arg(job_id.to_string())
            .arg("--details")
            .output()?
            .stdout,
    )?
    .split_whitespace()
    .map(|s| {
        let t = s.split('=').take(2).collect::<Vec<_>>();
        (t[0].to_string(), t[1].to_string())
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
            .expect("Cannot construct component. Please check your configuration!")
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

    let config = configuration::get_configuration()?;

    let client = AuditorClient::new(&config.addr, config.port)?;

    let job_id = get_slurm_job_id().expect("Collector not run in the context of a Slurm epilog");
    let job = get_slurm_job_info(job_id)?;

    // println!("{:?}", job);
    // println!("Server health: {}", client.health_check().await);

    let record = RecordAdd::new(
        format!("{}-{}", make_string_valid(&config.record_prefix), job_id),
        make_string_valid(&config.site_id),
        make_string_valid(&job["UserId"]),
        make_string_valid(&job["GroupId"]),
        construct_components(&config, &job),
        parse_slurm_timestamp(&job["StartTime"])?,
    )
    .expect("Could not construct record")
    .with_stop_time(parse_slurm_timestamp(&job["EndTime"])?);

    // println!("{:?}", record);

    client.add(record).await?;

    Ok(())
}
