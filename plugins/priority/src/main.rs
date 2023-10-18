// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anyhow::Error;
use auditor::client::AuditorClientBuilder;
use auditor::domain::Record;
use auditor::telemetry::{get_subscriber, init_subscriber};
use chrono::Utc;
use configuration::{ComputationMode, PrometheusMetricsOptions, Settings};
use num_traits::cast::FromPrimitive;
use std::collections::HashMap;
use std::net::TcpListener;
use std::process::Command;
use tracing::{debug, error, warn};
use uuid::Uuid;

mod configuration;
pub mod metrics;
mod startup;
use metrics::PrometheusExporterConfig;

use startup::run;

type ResourceName = String;
type ResourceValue = f64;
type PriorityName = String;
type PriorityValue = i64;

#[tracing::instrument(name = "Extracting resources from records", skip(records, config))]
fn extract(records: Vec<Record>, config: &Settings) -> HashMap<ResourceName, ResourceValue> {
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
                                                    (*s.value.as_ref(), true)
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
        if let Some(meta) = r.meta.as_ref() {
            if let Some(groups) = meta.get("group_id") {
                if let Some(group_id) = groups.get(0) {
                    // Only consider configured groups
                    if config.group_mapping.contains_key(group_id) {
                        // we know that the key exists (we filled it beforehand), therefore we can unwrap
                        *resources.get_mut(group_id).unwrap() += val;
                        println!("Resources: {resources:?}");
                    }
                }
            }
        } else {
            error!(record_id = %r.record_id, "Record without group_id, ignoring.");
        }
    }

    resources
}

#[tracing::instrument(name = "Computing priorities", skip(config))]
fn compute_priorities(
    resources: &HashMap<ResourceName, ResourceValue>,
    config: &Settings,
) -> HashMap<PriorityName, PriorityValue> {
    let (v_min, v_max, v_sum) = resources.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY, 0.0),
        |(cur_min, cur_max, sum), (_, v)| {
            (
                if *v < cur_min { *v } else { cur_min },
                if *v > cur_max { *v } else { cur_max },
                sum + *v,
            )
        },
    );

    let max_priority = f64::from_u64(config.max_priority).unwrap();
    let min_priority = f64::from_u64(config.min_priority).unwrap();

    match config.computation_mode {
        ComputationMode::FullSpread => resources
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    ((v - v_min) / (v_max - v_min) * (max_priority - min_priority) + min_priority)
                        .round() as i64,
                )
            })
            .collect(),
        ComputationMode::ScaledBySum => resources
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    ((max_priority - min_priority) / v_sum * v + min_priority).round() as i64,
                )
            })
            .collect(),
    }
}

#[tracing::instrument(name = "Constructing command for setting priorities")]
fn construct_command(
    cmd: &[String],
    priority: i64,
    resource: f64,
    group: &String,
    params: &[String],
) -> Vec<String> {
    cmd.iter()
        .map(|c| c.replace("{priority}", &format!("{priority}")))
        .map(|c| c.replace("{resource}", &format!("{resource}")))
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
fn set_priorities(
    priorities: &HashMap<PriorityName, PriorityValue>,
    resources: &HashMap<ResourceName, ResourceValue>,
    config: &Settings,
) -> Result<(), Error> {
    for command in config.commands.iter() {
        let command = shell_words::split(command)?;
        for (group, params) in config.group_mapping.iter() {
            // Only set priority if group actually exists.
            if let Some(prio) = priorities.get(group) {
                let resource = resources.get(group).unwrap();
                let command = construct_command(&command.clone(), *prio, *resource, group, params);

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
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = configuration::get_configuration()?;

    debug!(?config, "Loaded config");

    // Set up logging
    let subscriber = get_subscriber(
        "AUDITOR-priority-plugin".into(),
        config.log_level,
        std::io::stdout,
    );
    init_subscriber(subscriber);

    let run_id = Uuid::new_v4();
    let span = tracing::info_span!(
        "Running priority plugin",
        %run_id,
    );
    let _span_guard = span.enter();

    let client = AuditorClientBuilder::new()
        .address(&config.auditor.addr, config.auditor.port)
        .timeout(config.timeout)
        .build()?;

    let request_metrics = PrometheusExporterConfig::build()?;

    let cloned_request_metrics = request_metrics.clone();
    let mut interval = tokio::time::interval(config.frequency.to_std()?);
    let mut enable_prometheus = false;
    let mut prometheus_metrics = Vec::<PrometheusMetricsOptions>::new();

    match config.prometheus.clone() {
        Some(prometheus_settings) => {
            let prometheus_addr = prometheus_settings.addr.clone();
            let prometheus_port = prometheus_settings.port;
            enable_prometheus = prometheus_settings.enable;
            let address = format!("{}:{}", prometheus_addr, prometheus_port);
            prometheus_metrics = prometheus_settings.metrics;

            // Create a TcpListener for a given address and port
            let listener = TcpListener::bind(address)?;

            if enable_prometheus {
                tokio::spawn(run(listener, request_metrics).await?);
            }
        }
        None => {
            tracing::info!("Prometheus exporter is disabled");
        }
    };

    let main_task = tokio::spawn(async move {
        let configuration = config.clone();

        loop {
            tokio::select! {
                _ = interval.tick() => {

                let records = match config.duration {
                    Some(duration) => client
                        .get_stopped_since(&(Utc::now() - duration))
                        .await
                        .unwrap(),
                    None => client.get().await.unwrap(),
                };

                let resources = extract(records, &configuration);

                let priorities = compute_priorities(&resources, &configuration);

                let _ = set_priorities(&priorities, &resources, &configuration);


                     if enable_prometheus{
                         cloned_request_metrics
                             .update_prometheus_metrics(
                                 &resources,
                                 &priorities,
                                 &prometheus_metrics,
                             )
                             .await
                                 .unwrap();
                    }

                }

            }
        }
    });

    tokio::select! {
        _ = main_task => {
            tracing::info!("starting main task");
        }
        _ = tokio::signal::ctrl_c() => {
                    tracing::info!("CTRL-C received, shutting down priority plugin");
                }

    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::{AuditorSettings, PrometheusMetricsOptions, PrometheusSettings};
    use tracing_subscriber::filter::LevelFilter;

    #[test]
    fn test_compute_priorities_fullspread() {
        let resources = HashMap::from([
            ("blah1".to_string(), 2.0),
            ("blah3".to_string(), 4.0),
            ("blah2".to_string(), 3.0),
        ]);
        let config = Settings {
            auditor: AuditorSettings {
                addr: "whatever".to_string(),
                port: 1234,
            },
            timeout: 30,
            components: HashMap::new(),
            min_priority: 1,
            max_priority: 10,
            group_mapping: HashMap::new(),
            commands: vec!["whatever".to_string()],
            duration: None,
            computation_mode: ComputationMode::FullSpread,
            frequency: chrono::Duration::seconds(3600),
            log_level: LevelFilter::INFO,
            prometheus: Some(PrometheusSettings {
                enable: true,
                addr: "whatever".to_string(),
                port: 1234,
                metrics: vec![
                    PrometheusMetricsOptions::ResourceUsage,
                    PrometheusMetricsOptions::Priority,
                ],
            }),
        };

        let prios = compute_priorities(&resources, &config);

        assert_eq!(*prios.get("blah1").unwrap(), 1i64);
        assert_eq!(*prios.get("blah2").unwrap(), 6i64);
        assert_eq!(*prios.get("blah3").unwrap(), 10i64);
    }

    #[test]
    fn test_compute_priorities_scaledbysum() {
        let resources = HashMap::from([
            ("blah1".to_string(), 2.0),
            ("blah3".to_string(), 4.0),
            ("blah2".to_string(), 3.0),
        ]);
        let config = Settings {
            auditor: AuditorSettings {
                addr: "whatever".to_string(),
                port: 1234,
            },
            timeout: 30,
            components: HashMap::new(),
            min_priority: 1,
            max_priority: 10,
            group_mapping: HashMap::new(),
            commands: vec!["whatever".to_string()],
            duration: None,
            computation_mode: ComputationMode::ScaledBySum,
            frequency: chrono::Duration::seconds(3600),
            log_level: LevelFilter::INFO,
            prometheus: Some(PrometheusSettings {
                enable: true,
                addr: "whatever".to_string(),
                port: 1234,
                metrics: vec![
                    PrometheusMetricsOptions::ResourceUsage,
                    PrometheusMetricsOptions::Priority,
                ],
            }),
        };

        let prios = compute_priorities(&resources, &config);

        assert_eq!(*prios.get("blah1").unwrap(), 3i64);
        assert_eq!(*prios.get("blah2").unwrap(), 4i64);
        assert_eq!(*prios.get("blah3").unwrap(), 5i64);
    }

    #[test]
    fn test_construct_command() {
        let cmd = vec![
            "/usr/bin/scontrol".to_string(),
            "update".to_string(),
            "PartitionName={1}".to_string(),
            "PriorityFactor={priority}".to_string(),
            "SomeGroup={group}".to_string(),
            "SomeResourceStuff={resource}".to_string(),
            "SomethingElse={2}".to_string(),
        ];
        let priority = 10i64;
        let group = "atlas".to_string();
        let params = vec!["some_partition".to_string(), "blah".to_string()];
        let resource = 1.2;

        let cmd = construct_command(&cmd, priority, resource, &group, &params);
        assert_eq!(cmd[0], "/usr/bin/scontrol");
        assert_eq!(cmd[1], "update");
        assert_eq!(cmd[2], "PartitionName=some_partition");
        assert_eq!(cmd[3], "PriorityFactor=10");
        assert_eq!(cmd[4], "SomeGroup=atlas");
        assert_eq!(cmd[5], "SomeResourceStuff=1.2");
        assert_eq!(cmd[6], "SomethingElse=blah");
    }
}
