use anyhow::Error;
use auditor::client::AuditorClient;
use auditor::domain::{Component, RecordAdd, Score};
use auditor::telemetry::{get_subscriber, init_subscriber};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::HashMap;
use std::env;
use std::process::Command;

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

#[tracing::instrument(name = "Parsing Slurm timestamp")]
fn parse_slurm_timestamp<T: AsRef<str> + std::fmt::Debug>(
    timestamp: T,
) -> Result<DateTime<Utc>, Error> {
    Ok(DateTime::<Utc>::from_utc(
        NaiveDateTime::parse_from_str(timestamp.as_ref(), "%Y-%m-%dT%H:%M:%S")?,
        Utc,
    ))
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

    let auditor_host = "host.docker.internal";
    let auditor_port = 8000;

    let client = AuditorClient::new(&auditor_host, auditor_port)?;

    let job_id = get_slurm_job_id()?;
    let job_info = get_slurm_job_info(job_id)?;

    // println!("{:?}", job_info);
    println!("Server health: {}", client.health_check().await);

    let record_prefix = "slurm-".to_string();
    let site_id = "cluster1".to_string();
    let user_id = "user1".to_string();
    let group_id = "group1".to_string();

    let components = vec![Component::new(
        "Cores",
        job_info["NumCPUs"].parse()?,
        vec![Score::new("FakeHEPSPEC", 1.3).unwrap()],
    )
    .unwrap()];

    let record = RecordAdd::new(
        format!("{}{}", record_prefix, job_id),
        site_id,
        user_id,
        group_id,
        components,
        parse_slurm_timestamp(&job_info["StartTime"])?,
    )
    // Get rid of unwrap once rest of the library has proper error handling
    .unwrap()
    .with_stop_time(parse_slurm_timestamp(&job_info["EndTime"])?);

    println!("{:?}", record);

    client.add(record).await?;

    Ok(())
}
