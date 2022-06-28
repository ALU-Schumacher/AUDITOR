use anyhow::Error;
use auditor::client::AuditorClient;
// use auditor::constants::FORBIDDEN_CHARACTERS;
use auditor::domain::Record;
use auditor::telemetry::{get_subscriber, init_subscriber};
// use chrono::{DateTime, NaiveDateTime, Utc};
// use regex::Regex;
// use std::collections::HashMap;
// use std::env;
// use std::fmt;
use configuration::Settings;
use num_traits::cast::FromPrimitive;
use std::collections::HashMap;
use std::process::Command;
use tracing::{debug, error, info, warn};

mod configuration;

#[tracing::instrument(name = "Extracting resources from records")]
fn extract(records: Vec<Record>, config: &Settings) -> HashMap<String, f64> {
    if config.components.is_empty() {
        warn!(concat!(
            "Not configured how to extract metrics to account for ",
            "(components are missing). Will only account for time!"
        ));
    }

    let mut resources: HashMap<String, f64> = HashMap::new();

    for r in records {
        let val: f64 = if let Some(runtime) = r.runtime {
            f64::from_i64(runtime).unwrap()
                * if r.components.is_none() {
                    if !config.components.is_empty() {
                        error!(
                            record_id = %r.record_id,
                            "Unexpectetely no components in record. Ignoring record."
                        );
                        continue;
                    }
                    1.0
                } else {
                    match r.components.as_ref().unwrap().iter().fold(
                        (1.0, false),
                        |(acc, found), c| {
                            if config.components.contains_key(c.name.as_ref()) {
                                (
                                    acc * f64::from_i64(*c.amount.as_ref()).unwrap()
                                        * match c.scores.iter().fold(
                                            (1.0, false),
                                            |(acc, found), s| {
                                                if s.name.as_ref()
                                                    == config
                                                        .components
                                                        .get(c.name.as_ref())
                                                        .unwrap()
                                                {
                                                    (*s.factor.as_ref(), true)
                                                } else {
                                                    (acc, found)
                                                }
                                            },
                                        ) {
                                            (acc, true) => acc,
                                            (_, false) => {
                                                error!(
                                                    record_id = %r.record_id,
                                                    concat!(
                                                        "Did not find configured score ",
                                                        "in record! Assuming 1.0."
                                                    )
                                                );
                                                1.0
                                            }
                                        },
                                    true,
                                )
                            } else {
                                (acc, found)
                            }
                        },
                    ) {
                        (acc, true) => acc,
                        (_, false) => {
                            error!(
                                record_id = %r.record_id,
                                "Did not find configured components in record! Ignoring record."
                            );
                            continue;
                        }
                    }
                }
        } else {
            debug!(record_id = %r.record_id, "Record without runtime, ignoring.");
            continue;
        };
        // If not group id is present in the record, then record will be silently ignored
        if let Some(group_id) = r.group_id.as_ref() {
            if let Some(v) = resources.get_mut(group_id) {
                *v += val;
            } else {
                resources.insert(group_id.to_string(), val);
            }
        } else {
            debug!(record_id = %r.record_id, "Record without group_id, ignoring.");
        }
    }

    resources
}

#[tracing::instrument(name = "Computing priorities")]
fn compute_priorities(resources: HashMap<String, f64>, config: &Settings) -> HashMap<String, i64> {
    let (v_min, v_max) = resources.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(cur_min, cur_max), (_, v)| {
            (
                if *v < cur_min { *v } else { cur_min },
                if *v > cur_max { *v } else { cur_max },
            )
        },
    );

    let max_priority = f64::from_u64(config.max_priority).unwrap();
    let min_priority = f64::from_u64(config.min_priority).unwrap();

    resources
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                ((v - v_min) / (v_max - v_min) * (max_priority - min_priority) + min_priority)
                    .round() as i64,
            )
        })
        .collect()
}

#[tracing::instrument(name = "Constructing command for setting priorities")]
fn construct_command(cmd: &[String], priority: i64, params: &[String]) -> Vec<String> {
    cmd.iter()
        .map(|c| c.replace("{priority}", &format!("{}", priority)))
        .map(|c| {
            let mut cc = c;
            for (index, p) in params.iter().enumerate() {
                cc = cc.replace(&format!("{{{}}}", index + 1), p);
            }
            cc
        })
        .collect()
}

#[tracing::instrument(name = "Setting priorities")]
fn set_priorities(priorities: HashMap<String, i64>, config: &Settings) -> Result<(), Error> {
    let command = shell_words::split(&config.command)?;
    for (group, params) in config.group_mapping.iter() {
        let command = construct_command(&command.clone(), *priorities.get(group).unwrap(), params);
        // let mut cmd = command.clone();
        // for c in cmd.iter_mut() {
        //     *c = c.replace("{priority}", &format!("{}", priorities.get(group).unwrap()));
        // }
        //
        // for (index, p) in params.iter().enumerate() {
        //     for c in cmd.iter_mut() {
        //         *c = c.replace(&format!("{{{}}}", index), p);
        //     }
        // }

        let cmd_run = Command::new(&command[0])
            .args(&command[1..])
            .output()
            .map_err(|e| {
                error!("Setting priority failed!");
                e
            })?;
        let output = std::str::from_utf8(&cmd_run.stdout)?;
        debug!(command_output = %output, "Command output");
    }
    Ok(())
}
// &Command::new("/usr/bin/scontrol")
//     .arg("update")
//     .arg(format!("PartitionName={}", params[0]))
//     .arg(format!("PriorityFactor={}", priorities.get(group).unwrap()))
//     .output()?
//     .stdout,
// "sudo scontrol update PartitionName=<BLAH> PriorityJobFactor=<BLAH>"

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Set up logging
    let subscriber = get_subscriber(
        "AUDITOR-priority-plugin".into(),
        "info".into(),
        std::io::stdout,
    );
    init_subscriber(subscriber);

    info!("AUDITOR-priority-plugin started.");

    let config = configuration::get_configuration()?;

    debug!(?config, "Loaded config");

    let client = AuditorClient::new(&config.addr, config.port)?;

    set_priorities(
        compute_priorities(extract(client.get().await?, &config), &config),
        &config,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_command() {
        let cmd = vec![
            "/usr/bin/scontrol".to_string(),
            "update".to_string(),
            "PartitionName={1}".to_string(),
            "PriorityFactor={priority}".to_string(),
            "SomethingElse={2}".to_string(),
        ];
        let priority = 10i64;
        let params = vec!["some_partition".to_string(), "blah".to_string()];

        let cmd = construct_command(&cmd, priority, &params);
        assert_eq!(cmd[0], "/usr/bin/scontrol");
        assert_eq!(cmd[1], "update");
        assert_eq!(cmd[2], "PartitionName=some_partition");
        assert_eq!(cmd[3], "PriorityFactor=10");
        assert_eq!(cmd[4], "SomethingElse=blah");
    }
}
