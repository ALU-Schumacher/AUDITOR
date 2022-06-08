// use crate::slurm_epilog::configuration::get_configuration;
use anyhow::Error;
use auditor::client::AuditorClient;
use auditor::constants::FORBIDDEN_CHARACTERS;
use auditor::domain::{Component, RecordAdd, Score};
use auditor::telemetry::{get_subscriber, init_subscriber};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::process::Command;

mod configuration;

#[tracing::instrument(name = "Obtaining Slurm job id from environment")]
fn get_slurm_job_id() -> Result<u64, Error> {
    Ok(env::var("SLURM_JOB_ID")?.parse()?)
}

#[tracing::instrument(name = "Getting Slurm job info via scontrol")]
fn get_slurm_job_info(job_id: u64) -> Result<HashMap<String, String>, Error> {
    Ok(std::str::from_utf8(
        &Command::new("scontrol")
            .arg("show")
            .arg("job")
            .arg(job_id.to_string())
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

    let components = vec![Component::new(
        "Cores",
        job["NumCPUs"].parse()?,
        vec![Score::new("FakeHEPSPEC", 1.3).unwrap()],
    )
    .unwrap()];

    let record = RecordAdd::new(
        format!("{}-{}", make_string_valid(config.record_prefix), job_id),
        make_string_valid(config.site_id),
        make_string_valid(&job["UserId"]),
        make_string_valid(&job["GroupId"]),
        components,
        parse_slurm_timestamp(&job["StartTime"])?,
    )
    // Get rid of unwrap once rest of the library has proper error handling
    .unwrap()
    .with_stop_time(parse_slurm_timestamp(&job["EndTime"])?);

    // println!("{:?}", record);

    client.add(record).await?;

    Ok(())
}
