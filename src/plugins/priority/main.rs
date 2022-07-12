// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anyhow::Error;
use auditor::client::AuditorClient;
use auditor::domain::Record;
use auditor::telemetry::{get_subscriber, init_subscriber};
use configuration::Settings;
use num_traits::cast::FromPrimitive;
use std::collections::HashMap;
use std::process::Command;
use tracing::{debug, error, info, warn};

mod configuration;

#[tracing::instrument(name = "Extracting resources from records", skip(records, config))]
fn extract(records: Vec<Record>, config: &Settings) -> HashMap<String, f64> {
    if config.components.is_empty() {
        warn!(concat!(
            "Not configured how to extract metrics to account for ",
            "(components are missing). Will only account for time!"
        ));
    }

    let mut resources: HashMap<String, f64> = HashMap::new();

    for group in config.group_mapping.keys() {
        resources.insert(group.to_string(), 0.0);
    }

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
            error!(record_id = %r.record_id, "Record without runtime, ignoring.");
            continue;
        };
        // If no group_id is present in the record, then record will be silently ignored
        if let Some(group_id) = r.group_id.as_ref() {
            // Only consider configured groups
            if config.group_mapping.contains_key(group_id) {
                // we know that the key exists (we filled it beforehand), therefore we can unwrap
                *resources.get_mut(group_id).unwrap() += val;
            }
        } else {
            error!(record_id = %r.record_id, "Record without group_id, ignoring.");
        }
    }

    resources
}

#[tracing::instrument(name = "Computing priorities", skip(config))]
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
fn construct_command(
    cmd: &[String],
    priority: i64,
    group: &String,
    params: &[String],
) -> Vec<String> {
    cmd.iter()
        .map(|c| c.replace("{priority}", &format!("{}", priority)))
        .map(|c| c.replace("{group}", group))
        .map(|c| {
            let mut cc = c;
            for (index, p) in params.iter().enumerate() {
                cc = cc.replace(&format!("{{{}}}", index + 1), p);
            }
            cc
        })
        .collect()
}

#[tracing::instrument(name = "Setting priorities", skip(config))]
fn set_priorities(priorities: HashMap<String, i64>, config: &Settings) -> Result<(), Error> {
    for command in config.commands.iter() {
        let command = shell_words::split(command)?;
        for (group, params) in config.group_mapping.iter() {
            // Only set priority if group actually exists.
            if let Some(prio) = priorities.get(group) {
                let command = construct_command(&command.clone(), *prio, group, params);

                let mut cmd = Command::new(&command[0]);
                cmd.args(&command[1..]);

                debug!(?cmd, "Constructed command");

                let status = cmd.status().map_err(|e| {
                    error!("Executing command failed!");
                    e
                })?;

                debug!(?status, "Command status");

                if !status.success() {
                    error!("Setting priority failed!");
                }
                // let output = std::str::from_utf8(&cmd_run.stdout)?;
                // info!(command_output = %output, "Command output");
            }
        }
    }
    Ok(())
}

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
    fn test_compute_priorities() {
        let resources = HashMap::from([
            ("blah1".to_string(), 2.0),
            ("blah3".to_string(), 4.0),
            ("blah2".to_string(), 3.0),
        ]);
        let config = Settings {
            addr: "whatever".to_string(),
            port: 1234,
            components: HashMap::new(),
            min_priority: 1,
            max_priority: 10,
            group_mapping: HashMap::new(),
            commands: vec!["whatever".to_string()],
        };

        let prios = compute_priorities(resources, &config);

        assert_eq!(*prios.get("blah1").unwrap(), 1i64);
        assert_eq!(*prios.get("blah2").unwrap(), 6i64);
        assert_eq!(*prios.get("blah3").unwrap(), 10i64);
    }

    #[test]
    fn test_construct_command() {
        let cmd = vec![
            "/usr/bin/scontrol".to_string(),
            "update".to_string(),
            "PartitionName={1}".to_string(),
            "PriorityFactor={priority}".to_string(),
            "SomeGroup={group}".to_string(),
            "SomethingElse={2}".to_string(),
        ];
        let priority = 10i64;
        let group = "atlas".to_string();
        let params = vec!["some_partition".to_string(), "blah".to_string()];

        let cmd = construct_command(&cmd, priority, &group, &params);
        assert_eq!(cmd[0], "/usr/bin/scontrol");
        assert_eq!(cmd[1], "update");
        assert_eq!(cmd[2], "PartitionName=some_partition");
        assert_eq!(cmd[3], "PriorityFactor=10");
        assert_eq!(cmd[4], "SomeGroup=atlas");
        assert_eq!(cmd[5], "SomethingElse=blah");
    }
}
