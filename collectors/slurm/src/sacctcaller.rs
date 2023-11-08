// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::{collections::HashMap, fmt};

use anyhow::anyhow;
use auditor::{
    constants::FORBIDDEN_CHARACTERS,
    domain::{Component, RecordAdd, Score},
};
use chrono::{DateTime, FixedOffset, Local, Utc};
use color_eyre::eyre::{eyre, Result};
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::{process::Command, sync::mpsc};

use crate::{
    configuration::{AllowedTypes, ComponentConfig, KeyConfig, ParsableType, Settings},
    database::Database,
    shutdown::Shutdown,
    CONFIG, END, GROUP, JOBID, KEYS, START, USER,
};

type SacctRow = HashMap<String, Option<AllowedTypes>>;
type SacctRows = HashMap<String, SacctRow>;
type Job = HashMap<String, AllowedTypes>;

static BATCH_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9_]+\.batch$")
        .expect("Could not construct essential Regex for matching job ids.")
});

static SUB_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9]+\.[0-9]*$")
        .expect("Could not construct essential Regex for matching job ids.")
});

#[tracing::instrument(
    name = "Starting sacct monitor",
    skip(database, tx, _shutdown_notifier, shutdown, hold_till_shutdown)
)]
pub(crate) async fn run_sacct_monitor(
    database: Database,
    tx: mpsc::Sender<RecordAdd>,
    _shutdown_notifier: mpsc::UnboundedSender<()>,
    mut shutdown: Shutdown,
    hold_till_shutdown: mpsc::Sender<()>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CONFIG.sacct_frequency.to_std().unwrap());
        loop {
            tokio::select! {
                _ = interval.tick() => {},
                _ = shutdown.recv() => {
                    tracing::info!("Sacct monitor received shutdown signal. Shutting down.");
                    // shutdown properly
                    drop(hold_till_shutdown);
                    break
                },
            }
            tokio::select! {
                records = get_job_info(&database) => {
                    match records {
                        Ok(records) => place_records_on_queue(records, &tx).await,
                        Err(e) => {
                            tracing::error!("something went wrong: {:?}", e);
                            continue
                        }
                    };
                },
                _ = shutdown.recv() => {
                    tracing::info!("Sacct monitor received shutdown signal. Shutting down.");
                    // shutdown properly
                    drop(hold_till_shutdown);
                    break
                },
            }
        }
    });
}

#[tracing::instrument(name = "Placing records on queue", level = "debug", skip(records, tx))]
async fn place_records_on_queue(records: Vec<RecordAdd>, tx: &mpsc::Sender<RecordAdd>) {
    for record in records {
        let record_id = record.record_id.clone();
        if let Err(e) = tx.send(record).await {
            tracing::error!("Could not send record {:?} to queue: {:?}", record_id, e);
        }
    }
}

#[tracing::instrument(name = "Calling sacct and parsing output", skip(database))]
async fn get_job_info(database: &Database) -> Result<Vec<RecordAdd>> {
    let (lastcheck, last_record_id) = database.get_lastcheck().await?;

    let binary = "/usr/bin/sacct";
    let mut args = vec![
        "-a".to_string(),
        "--format".to_string(),
        KEYS.iter().map(|k| k.name.clone()).join(","),
        "--noconvert".to_string(),
        "--noheader".to_string(),
        "-S".to_string(),
        format!("{}", lastcheck.format("%Y-%m-%dT%H:%M:%S")),
        "-E".to_string(),
        "now".to_string(),
        "-P".to_string(),
    ];

    if !CONFIG.job_filter.status.is_empty() {
        args.push("-s".to_string());
        args.push(CONFIG.job_filter.status.join(","));
    }

    if !CONFIG.job_filter.partition.is_empty() {
        args.push("-r".to_string());
        args.push(CONFIG.job_filter.partition.join(","));
    }

    if !CONFIG.job_filter.user.is_empty() {
        args.push("-u".to_string());
        args.push(CONFIG.job_filter.user.join(","));
    }

    if !CONFIG.job_filter.group.is_empty() {
        args.push("-g".to_string());
        args.push(CONFIG.job_filter.group.join(","));
    }

    if !CONFIG.job_filter.account.is_empty() {
        args.push("-A".to_string());
        args.push(CONFIG.job_filter.account.join(","));
    }

    let cmd = binary.to_owned() + " " + &args.join(" ");
    tracing::debug!("Executing the following command: {}", cmd);

    let cmd_out = Command::new(binary).args(&args).output().await?;

    let cmd_out = std::str::from_utf8(&cmd_out.stdout)?;
    tracing::debug!("Got: {}", cmd_out);

    let sacct_rows = tokenize_sacct_output(cmd_out, KEYS.to_vec());
    let parsed_sacct_rows = parse_sacct_rows(sacct_rows, &KEYS.to_vec())?;
    let records = parsed_sacct_rows
        .iter()
        .map(|map| construct_record(map, &last_record_id, &CONFIG))
        .collect::<Result<Vec<Option<RecordAdd>>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let (nextcheck, rid) = if records.is_empty() {
        (lastcheck, last_record_id)
    } else {
        let local_offset = Local::now().offset().utc_minus_local();
        let (ts, rid) = records.iter().fold(
            (chrono::DateTime::<Utc>::MIN_UTC, String::new()),
            |(acc, _acc_record_id), r| {
                (
                    acc.max(r.stop_time.unwrap()),
                    r.record_id.as_ref().to_string(),
                )
            },
        );
        (
            DateTime::<Local>::from_naive_utc_and_offset(
                ts.naive_utc(),
                FixedOffset::east_opt(local_offset).unwrap(),
            ),
            rid,
        )
    };

    database.set_lastcheck(rid, nextcheck).await?;

    Ok(records)
}

#[tracing::instrument(name = "Tokenizing sacct output", skip(keys))]
fn tokenize_sacct_output(output: &str, keys: Vec<KeyConfig>) -> SacctRows {
    output
        .lines()
        .map(|l| {
            keys.iter()
                .cloned()
                .zip(l.split('|').map(|s| s.to_owned()))
                // Occasionally fields are empty by design. filter those out to avoid
                // problems later on when parsing.
                .filter(|(kc, v)| !v.is_empty() || kc.allow_empty)
                .map(|(kc, v)| {
                    let v = match kc.key_type.parse(&v) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!(
                                "Parsing '{}' (key: {}) as {:?} failed: {:?}. This may or may not be a problem. It probably is.",
                                v,
                                kc.name,
                                kc.key_type,
                                e
                            );
                            None
                        }
                    };
                    (kc.name, v)
                })
                .collect::<SacctRow>()
        })
        .map(|hm| (hm[JOBID].as_ref().unwrap().extract_string().unwrap(), hm))
        .collect::<SacctRows>()
}

#[tracing::instrument(name = "Parse sacct rows")]
fn parse_sacct_rows(sacct_rows: SacctRows, keys: &Vec<KeyConfig>) -> Result<Vec<Job>> {
    sacct_rows
        .keys()
        .filter(|k| !BATCH_REGEX.is_match(k))
        .filter(|k| !SUB_REGEX.is_match(k))
        .map(|id| -> Result<Job> {
            let map1 = sacct_rows.get(id).ok_or(eyre!("Cannot get map1"))?;
            let map2 = sacct_rows.get(&format!("{id}.batch"));
            Ok(keys.iter()
                .cloned()
                .filter_map(|KeyConfig {name: k, key_type: _, allow_empty: _}| {
                    let val = match map1.get(&k) {
                        Some(Some(v)) => Some(v.clone()),
                        _ => {
                            if let Some(map2) = map2 {
                                match map2.get(&k) {
                                    Some(Some(v)) => Some(v.clone()),
                                    _ => {
                                        tracing::error!("Something went wrong during parsing (map1, id: {id}, key: {k}, value: {:?})", map2.get(&k));
                                        None
                                    },
                                }
                            } else {
                                tracing::error!("Something went wrong during parsing (map2, id: {id}, key: {k}, value: {:?})", map1.get(&k));
                                None
                            }
                        },
                    };
                    val.map(|val| (k, val))
                })
                .collect::<Job>())
        }).collect::<Result<Vec<Job>>>()
}

#[tracing::instrument(name = "Construct record", level = "debug")]
fn construct_record(
    map: &Job,
    last_record_id: &str,
    config: &Settings,
) -> Result<Option<RecordAdd>> {
    let job_id = map[JOBID].extract_string()?;
    let site = if let Some(site) = identify_site(map) {
        site
    } else {
        tracing::warn!(
                "No configured site matched for job {}! Ignoring job. Consider adding a match-all at the end of the sites configuration.",
                job_id
            );
        return Ok(None);
    };

    let record_id = make_string_valid(format!("{}-{job_id}", &CONFIG.record_prefix));
    // We don't want this record, we have already seen it in a previous run.
    if record_id == last_record_id {
        return Ok(None);
    }

    let mut meta = if let Some(ref meta) = CONFIG.meta {
        meta.iter()
            .map(|m| -> Result<Vec<(String, Vec<String>)>> {
                let map = if m.key_type == ParsableType::Json {
                    if let Some(val) = map.get(&m.key) {
                        val.extract_map()?
                            .iter()
                            .map(|(k, v)| -> Result<(String, Vec<String>)> {
                                Ok((
                                    make_string_valid(k.extract_string()?),
                                    vec![make_string_valid(v.extract_string()?)],
                                ))
                            })
                            .collect::<Result<Vec<(_, _)>>>()?
                    } else {
                        vec![]
                    }
                } else {
                    vec![(
                        m.name.clone(),
                        vec![make_string_valid(map[&m.key].extract_as_string()?)],
                    )]
                };
                Ok(map)
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flat_map(|m| m.into_iter())
            .collect::<HashMap<_, _>>()
    } else {
        HashMap::new()
    };

    meta.insert("site_id".to_string(), vec![make_string_valid(site)]);
    meta.insert(
        "user_id".to_string(),
        vec![make_string_valid(map[USER].extract_string()?)],
    );
    meta.insert(
        "group_id".to_string(),
        vec![make_string_valid(map[GROUP].extract_string()?)],
    );

    let components = if let Ok(components) = construct_components(map, &config.components) {
        components
    } else {
        tracing::warn!(
            "Could not construct components for job {}. This job will be ignored.",
            job_id
        );
        return Ok(None);
    };

    Ok(Some(
        RecordAdd::new(record_id, meta, components, map[START].extract_datetime()?)
            .expect("Could not construct record")
            .with_stop_time(map[END].extract_datetime()?),
    ))
}

#[tracing::instrument(name = "Remove forbidden characters from string", level = "debug")]
fn make_string_valid<T: AsRef<str> + fmt::Debug>(input: T) -> String {
    input.as_ref().replace(&FORBIDDEN_CHARACTERS[..], "")
}

#[tracing::instrument(name = "Obtain site from job info and configuration", level = "debug")]
fn identify_site(job: &Job) -> Option<String> {
    CONFIG
        .sites
        .iter()
        .filter(|s| {
            s.only_if.is_none() || {
                let only_if = s.only_if.as_ref().unwrap();
                let re = Regex::new(&only_if.matches)
                    .unwrap_or_else(|_| panic!("Invalid regex expression: {}", &only_if.matches));
                re.is_match(&job[&only_if.key].extract_string().unwrap_or_else(|_| {
                    panic!("Key is expected to be a string: {:?}", job[&only_if.key])
                }))
            }
        })
        .cloned()
        .map(|s| make_string_valid(s.name))
        .collect::<Vec<_>>()
        .get(0)
        .cloned()
}

#[tracing::instrument(
    name = "Construct components from job info and configuration",
    level = "debug",
    skip(components_config)
)]
fn construct_components(
    job: &Job,
    components_config: &[ComponentConfig],
) -> Result<Vec<Component>, anyhow::Error> {
    components_config
        .iter()
        .filter(|c| {
            c.only_if.is_none() || {
                let only_if = c.only_if.as_ref().unwrap();
                let re = Regex::new(&only_if.matches)
                    .unwrap_or_else(|_| panic!("Invalid regex expression: {}", &only_if.matches));
                re.is_match(&job[&only_if.key].extract_string().unwrap_or_else(|_| {
                    panic!("Key is expected to be a string: {:?}", job[&only_if.key])
                }))
            }
        })
        .cloned()
        .map(|c| {
            if !job.contains_key(&c.key) {
                if let Some(default_value) = c.default_value {
                    Ok(Component::new(make_string_valid(&c.name), default_value)
                        .expect("Cannot construct component")
                        .with_scores(construct_component_scores(job, &c)))
                } else {
                    // TODO we should probably create our own error type (enum) and return it here
                    // maybe this error type can also be used in other parts of this function
                    Err(anyhow!("Job information does not contain key {}", &c.key))
                }
            } else {
                Ok(Component::new(
                    make_string_valid(&c.name),
                    job[&c.key].extract_i64().unwrap_or_else(|_| {
                        panic!(
                            "Cannot parse key {} (value: {:?}) into i64.",
                            c.key, job[&c.key]
                        )
                    }),
                )
                .expect("Cannot construct component.")
                .with_scores(construct_component_scores(job, &c)))
            }
        })
        .collect()
}

fn construct_component_scores(job: &Job, component_config: &ComponentConfig) -> Vec<Score> {
    component_config
        .scores
        .iter()
        .filter(|s| {
            s.only_if.is_none() || {
                let only_if = s.only_if.as_ref().unwrap();
                let re = Regex::new(&only_if.matches)
                    .unwrap_or_else(|_| panic!("Invalid regex expression: {}", &only_if.matches));
                re.is_match(
                    &job[&only_if.key]
                        .extract_string()
                        .unwrap_or_else(|_| panic!("Error extracting string.")),
                )
            }
        })
        .map(|s| {
            Score::new(s.name.clone(), s.value)
                .unwrap_or_else(|_| panic!("Cannot construct score from {s:?}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use auditor::domain::{ValidAmount, ValidName, ValidValue};
    use chrono::NaiveDateTime;

    use super::*;
    use crate::{
        configuration::{OnlyIf, ScoreConfig},
        STATE,
    };

    #[test]
    fn match_job_ids() {
        assert!(BATCH_REGEX.is_match("1234.batch"));
        assert!(BATCH_REGEX.is_match("1234_10.batch"));
        assert!(SUB_REGEX.is_match("123.456"));
    }

    #[test]
    fn tokenize_sacct_output_common_usecase_succeeds() {
        let keys = vec![
            KeyConfig {
                name: "Partition".to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: "NCPUS".to_owned(),
                key_type: ParsableType::Integer,
                allow_empty: false,
            },
            KeyConfig {
                name: "ReqMem".to_owned(),
                key_type: ParsableType::IntegerMega,
                allow_empty: false,
            },
            KeyConfig {
                name: "NNodes".to_owned(),
                key_type: ParsableType::Integer,
                allow_empty: false,
            },
            KeyConfig {
                name: JOBID.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: START.to_owned(),
                key_type: ParsableType::DateTime,
                allow_empty: false,
            },
            KeyConfig {
                name: END.to_owned(),
                key_type: ParsableType::DateTime,
                allow_empty: false,
            },
            KeyConfig {
                name: GROUP.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: USER.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: STATE.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
        ];
        let sacct_output = "partition|1|2000M|1|1234567|2023-11-07T10:14:01|2023-11-07T11:39:09|group|user|COMPLETED";
        let sacct_rows = tokenize_sacct_output(sacct_output, keys);
        let expected = SacctRows::from([(
            "1234567".to_owned(),
            SacctRow::from([
                (
                    "Partition".to_owned(),
                    Some(AllowedTypes::String("partition".to_owned())),
                ),
                ("NCPUS".to_owned(), Some(AllowedTypes::Integer(1))),
                ("ReqMem".to_owned(), Some(AllowedTypes::Integer(2000))),
                ("NNodes".to_owned(), Some(AllowedTypes::Integer(1))),
                (
                    JOBID.to_owned(),
                    Some(AllowedTypes::String("1234567".to_owned())),
                ),
                (
                    START.to_owned(),
                    Some(AllowedTypes::DateTime(DateTime::<Utc>::from(
                        NaiveDateTime::parse_from_str("2023-11-07T10:14:01", "%Y-%m-%dT%H:%M:%S")
                            .unwrap()
                            .and_local_timezone(
                                FixedOffset::east_opt(Local::now().offset().local_minus_utc())
                                    .unwrap(),
                            )
                            .unwrap(),
                    ))),
                ),
                (
                    END.to_owned(),
                    Some(AllowedTypes::DateTime(DateTime::<Utc>::from(
                        NaiveDateTime::parse_from_str("2023-11-07T11:39:09", "%Y-%m-%dT%H:%M:%S")
                            .unwrap()
                            .and_local_timezone(
                                FixedOffset::east_opt(Local::now().offset().local_minus_utc())
                                    .unwrap(),
                            )
                            .unwrap(),
                    ))),
                ),
                (
                    USER.to_owned(),
                    Some(AllowedTypes::String("user".to_owned())),
                ),
                (
                    GROUP.to_owned(),
                    Some(AllowedTypes::String("group".to_owned())),
                ),
                (
                    STATE.to_owned(),
                    Some(AllowedTypes::String("COMPLETED".to_owned())),
                ),
            ]),
        )]);

        assert_eq!(sacct_rows, expected);
    }

    #[test]
    fn tokenize_sacct_output_json_empty_string_allow() {
        let keys = vec![
            KeyConfig {
                name: JOBID.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: "Comment".to_owned(),
                key_type: ParsableType::Json,
                allow_empty: true,
            },
            KeyConfig {
                name: STATE.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
        ];
        let sacct_output = "100||COMPLETED";
        let sacct_rows = tokenize_sacct_output(sacct_output, keys);

        let expected = SacctRows::from([(
            "100".to_owned(),
            SacctRow::from([
                (
                    JOBID.to_owned(),
                    Some(AllowedTypes::String("100".to_owned())),
                ),
                (
                    STATE.to_owned(),
                    Some(AllowedTypes::String("COMPLETED".to_owned())),
                ),
                ("Comment".to_owned(), Some(AllowedTypes::Map(vec![]))),
            ]),
        )]);

        assert_eq!(sacct_rows, expected);
    }

    #[test]
    fn tokenize_sacct_output_json_empty_string_disallow() {
        let keys = vec![
            KeyConfig {
                name: JOBID.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: "Comment".to_owned(),
                key_type: ParsableType::Json,
                allow_empty: false,
            },
            KeyConfig {
                name: STATE.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
        ];
        let sacct_output = "100||COMPLETED";
        let sacct_rows = tokenize_sacct_output(sacct_output, keys);

        let expected = SacctRows::from([(
            "100".to_owned(),
            SacctRow::from([
                (
                    JOBID.to_owned(),
                    Some(AllowedTypes::String("100".to_owned())),
                ),
                (
                    STATE.to_owned(),
                    Some(AllowedTypes::String("COMPLETED".to_owned())),
                ),
            ]),
        )]);

        assert_eq!(sacct_rows, expected);
    }

    #[test]
    fn tokenize_sacct_output_json_empty_json() {
        let keys = vec![
            KeyConfig {
                name: JOBID.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: "Comment".to_owned(),
                key_type: ParsableType::Json,
                allow_empty: false,
            },
            KeyConfig {
                name: STATE.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
        ];
        let sacct_output = "100|{}|COMPLETED";
        let sacct_rows = tokenize_sacct_output(sacct_output, keys);

        let expected = SacctRows::from([(
            "100".to_owned(),
            SacctRow::from([
                (
                    JOBID.to_owned(),
                    Some(AllowedTypes::String("100".to_owned())),
                ),
                (
                    STATE.to_owned(),
                    Some(AllowedTypes::String("COMPLETED".to_owned())),
                ),
                ("Comment".to_owned(), Some(AllowedTypes::Map(vec![]))),
            ]),
        )]);

        assert_eq!(sacct_rows, expected);
    }

    #[test]
    fn tokenize_sacct_output_json_full_json() {
        let keys = vec![
            KeyConfig {
                name: JOBID.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: "Comment".to_owned(),
                key_type: ParsableType::Json,
                allow_empty: false,
            },
            KeyConfig {
                name: STATE.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
        ];
        let sacct_output = "100|{ 'key': 'value' }|COMPLETED";
        let sacct_rows = tokenize_sacct_output(sacct_output, keys);

        let expected = SacctRows::from([(
            "100".to_owned(),
            SacctRow::from([
                (
                    JOBID.to_owned(),
                    Some(AllowedTypes::String("100".to_owned())),
                ),
                (
                    STATE.to_owned(),
                    Some(AllowedTypes::String("COMPLETED".to_owned())),
                ),
                (
                    "Comment".to_owned(),
                    Some(AllowedTypes::Map(vec![(
                        AllowedTypes::String("key".to_owned()),
                        AllowedTypes::String("value".to_owned()),
                    )])),
                ),
            ]),
        )]);

        assert_eq!(sacct_rows, expected);
    }

    #[test]
    fn parse_sacct_rows_empty_succeeds() {
        let keys = vec![
            KeyConfig {
                name: JOBID.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: STATE.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
        ];

        let sacct_rows = SacctRows::from([]);
        let parsed_sacct_rows = parse_sacct_rows(sacct_rows, &keys).unwrap();

        let expected = vec![];
        assert_eq!(parsed_sacct_rows, expected);
    }

    #[test]
    fn parse_sacct_rows_default_usecase_succeeds() {
        let keys = vec![
            KeyConfig {
                name: "Partition".to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: "NCPUS".to_owned(),
                key_type: ParsableType::Integer,
                allow_empty: false,
            },
            KeyConfig {
                name: "ReqMem".to_owned(),
                key_type: ParsableType::IntegerMega,
                allow_empty: false,
            },
            KeyConfig {
                name: "NNodes".to_owned(),
                key_type: ParsableType::Integer,
                allow_empty: false,
            },
            KeyConfig {
                name: "MaxRSS".to_owned(),
                key_type: ParsableType::Integer,
                allow_empty: false,
            },
            KeyConfig {
                name: JOBID.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: START.to_owned(),
                key_type: ParsableType::DateTime,
                allow_empty: false,
            },
            KeyConfig {
                name: END.to_owned(),
                key_type: ParsableType::DateTime,
                allow_empty: false,
            },
            KeyConfig {
                name: GROUP.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: USER.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
            KeyConfig {
                name: STATE.to_owned(),
                key_type: ParsableType::String,
                allow_empty: false,
            },
        ];

        // Slurm always returns two rows for each job.
        // The first line contains the normal job ID and most information
        // The second line contains the "<jobid>.batch" job id.
        // Here, the some information like user, group, partition, or ReqMem is missing.
        // However, the second line contains information such as MaxRSS
        let sacct_rows = SacctRows::from([
            (
                "1234567".to_owned(),
                SacctRow::from([
                    (
                        "Partition".to_owned(),
                        Some(AllowedTypes::String("partition".to_owned())),
                    ),
                    ("NCPUS".to_owned(), Some(AllowedTypes::Integer(1))),
                    ("ReqMem".to_owned(), Some(AllowedTypes::Integer(2000))),
                    ("NNodes".to_owned(), Some(AllowedTypes::Integer(1))),
                    (
                        JOBID.to_owned(),
                        Some(AllowedTypes::String("1234567".to_owned())),
                    ),
                    (
                        START.to_owned(),
                        Some(AllowedTypes::DateTime(DateTime::<Utc>::from(
                            NaiveDateTime::parse_from_str(
                                "2023-11-07T10:14:01",
                                "%Y-%m-%dT%H:%M:%S",
                            )
                            .unwrap()
                            .and_local_timezone(
                                FixedOffset::east_opt(Local::now().offset().local_minus_utc())
                                    .unwrap(),
                            )
                            .unwrap(),
                        ))),
                    ),
                    (
                        END.to_owned(),
                        Some(AllowedTypes::DateTime(DateTime::<Utc>::from(
                            NaiveDateTime::parse_from_str(
                                "2023-11-07T11:39:09",
                                "%Y-%m-%dT%H:%M:%S",
                            )
                            .unwrap()
                            .and_local_timezone(
                                FixedOffset::east_opt(Local::now().offset().local_minus_utc())
                                    .unwrap(),
                            )
                            .unwrap(),
                        ))),
                    ),
                    (
                        USER.to_owned(),
                        Some(AllowedTypes::String("user".to_owned())),
                    ),
                    (
                        GROUP.to_owned(),
                        Some(AllowedTypes::String("group".to_owned())),
                    ),
                    (
                        STATE.to_owned(),
                        Some(AllowedTypes::String("COMPLETED".to_owned())),
                    ),
                ]),
            ),
            (
                "1234567.batch".to_owned(),
                SacctRow::from([
                    ("NCPUS".to_owned(), Some(AllowedTypes::Integer(1))),
                    ("NNodes".to_owned(), Some(AllowedTypes::Integer(1))),
                    ("MaxRSS".to_owned(), Some(AllowedTypes::Integer(1_000_000))),
                    (
                        JOBID.to_owned(),
                        Some(AllowedTypes::String("1234567.batch".to_owned())),
                    ),
                    (
                        START.to_owned(),
                        Some(AllowedTypes::DateTime(DateTime::<Utc>::from(
                            NaiveDateTime::parse_from_str(
                                "2023-11-07T10:14:01",
                                "%Y-%m-%dT%H:%M:%S",
                            )
                            .unwrap()
                            .and_local_timezone(
                                FixedOffset::east_opt(Local::now().offset().local_minus_utc())
                                    .unwrap(),
                            )
                            .unwrap(),
                        ))),
                    ),
                    (
                        END.to_owned(),
                        Some(AllowedTypes::DateTime(DateTime::<Utc>::from(
                            NaiveDateTime::parse_from_str(
                                "2023-11-07T11:39:09",
                                "%Y-%m-%dT%H:%M:%S",
                            )
                            .unwrap()
                            .and_local_timezone(
                                FixedOffset::east_opt(Local::now().offset().local_minus_utc())
                                    .unwrap(),
                            )
                            .unwrap(),
                        ))),
                    ),
                    (
                        STATE.to_owned(),
                        Some(AllowedTypes::String("COMPLETED".to_owned())),
                    ),
                ]),
            ),
        ]);

        let parsed_sacct_rows = parse_sacct_rows(sacct_rows, &keys).unwrap();

        let expected = vec![Job::from([
            (
                "Partition".to_owned(),
                AllowedTypes::String("partition".to_owned()),
            ),
            ("NCPUS".to_owned(), AllowedTypes::Integer(1)),
            ("ReqMem".to_owned(), AllowedTypes::Integer(2000)),
            ("MaxRSS".to_owned(), AllowedTypes::Integer(1_000_000)),
            ("NNodes".to_owned(), AllowedTypes::Integer(1)),
            (
                "JobID".to_owned(),
                AllowedTypes::String("1234567".to_owned()),
            ),
            (
                START.to_owned(),
                AllowedTypes::DateTime(DateTime::<Utc>::from(
                    NaiveDateTime::parse_from_str("2023-11-07T10:14:01", "%Y-%m-%dT%H:%M:%S")
                        .unwrap()
                        .and_local_timezone(
                            FixedOffset::east_opt(Local::now().offset().local_minus_utc()).unwrap(),
                        )
                        .unwrap(),
                )),
            ),
            (
                END.to_owned(),
                AllowedTypes::DateTime(DateTime::<Utc>::from(
                    NaiveDateTime::parse_from_str("2023-11-07T11:39:09", "%Y-%m-%dT%H:%M:%S")
                        .unwrap()
                        .and_local_timezone(
                            FixedOffset::east_opt(Local::now().offset().local_minus_utc()).unwrap(),
                        )
                        .unwrap(),
                )),
            ),
            (USER.to_owned(), AllowedTypes::String("user".to_owned())),
            (GROUP.to_owned(), AllowedTypes::String("group".to_owned())),
            (
                STATE.to_owned(),
                AllowedTypes::String("COMPLETED".to_owned()),
            ),
        ])];
        assert_eq!(parsed_sacct_rows, expected);
    }

    #[test]
    fn construct_components_empty_config_succeeds() {
        let job = Job::from([(
            "JobID".to_owned(),
            AllowedTypes::String("6776554".to_owned()),
        )]);
        let components_config = vec![];
        let components = construct_components(&job, &components_config).unwrap();

        let expected: Vec<Component> = vec![];

        assert_eq!(components, expected);
    }

    #[test]
    fn construct_components_succeeds() {
        let job = Job::from([
            (
                "JobID".to_owned(),
                AllowedTypes::String("6776554".to_owned()),
            ),
            ("MaxRSS".to_owned(), AllowedTypes::Integer(1024)),
        ]);
        let components_config = vec![ComponentConfig {
            name: "MaxRSS".to_owned(),
            key: "MaxRSS".to_owned(),
            key_type: ParsableType::Integer,
            key_allow_empty: false,
            default_value: None,
            scores: vec![],
            only_if: None,
        }];
        let components = construct_components(&job, &components_config).unwrap();

        let expected = vec![Component {
            name: ValidName::parse("MaxRSS".to_owned()).unwrap(),
            amount: ValidAmount::parse(1024).unwrap(),
            scores: vec![],
        }];

        assert_eq!(components, expected);
    }

    #[test]
    fn construct_components_missing_key_fails() {
        let job = Job::from([(
            "JobID".to_owned(),
            AllowedTypes::String("6776554".to_owned()),
        )]);
        let components_config = vec![ComponentConfig {
            name: "MaxRSS".to_owned(),
            key: "MaxRSS".to_owned(),
            key_type: ParsableType::Integer,
            key_allow_empty: false,
            default_value: None,
            scores: vec![],
            only_if: None,
        }];
        // TODO we should probably test for the specific error
        // see https://zhauniarovich.com/post/2021/2021-01-testing-errors-in-rust/
        assert!(construct_components(&job, &components_config).is_err());
    }

    #[test]
    fn construct_components_default_value_is_used() {
        let job = Job::from([(
            "JobID".to_owned(),
            AllowedTypes::String("6776554".to_owned()),
        )]);
        let components_config = vec![ComponentConfig {
            name: "MaxRSS".to_owned(),
            key: "MaxRSS".to_owned(),
            key_type: ParsableType::Integer,
            key_allow_empty: false,
            default_value: Some(0),
            scores: vec![],
            only_if: None,
        }];
        let components = construct_components(&job, &components_config).unwrap();

        let expected = vec![Component {
            name: ValidName::parse("MaxRSS".to_owned()).unwrap(),
            amount: ValidAmount::parse(0).unwrap(),
            scores: vec![],
        }];

        assert_eq!(components, expected);
    }

    #[test]
    fn construct_component_scores_multiple_scores_succeeds() {
        let job = Job::from([
            (
                "JobID".to_owned(),
                AllowedTypes::String("1234567".to_owned()),
            ),
            ("NCPUS".to_owned(), AllowedTypes::Integer(8)),
        ]);

        let component_config = ComponentConfig {
            name: "NCPUS".to_owned(),
            key: "NCPUS".to_owned(),
            key_type: ParsableType::Integer,
            key_allow_empty: false,
            default_value: None,
            scores: vec![
                ScoreConfig {
                    name: "HEPSPEC06".to_owned(),
                    value: 10.0,
                    only_if: None,
                },
                ScoreConfig {
                    name: "hepscore23".to_owned(),
                    value: 10.0,
                    only_if: None,
                },
            ],
            only_if: None,
        };

        let component_scores = construct_component_scores(&job, &component_config);

        let expected = vec![
            Score {
                name: ValidName::parse("HEPSPEC06".to_owned()).unwrap(),
                value: ValidValue::parse(10.0).unwrap(),
            },
            Score {
                name: ValidName::parse("hepscore23".to_owned()).unwrap(),
                value: ValidValue::parse(10.0).unwrap(),
            },
        ];

        assert_eq!(component_scores, expected);
    }

    #[test]
    fn construct_component_scores_with_only_if_succeeds() {
        let job_1 = Job::from([
            (
                "JobID".to_owned(),
                AllowedTypes::String("1234567".to_owned()),
            ),
            ("NCPUS".to_owned(), AllowedTypes::Integer(8)),
            (
                "Partition".to_owned(),
                AllowedTypes::String("partition1".to_owned()),
            ),
        ]);
        let job_2 = Job::from([
            (
                "JobID".to_owned(),
                AllowedTypes::String("1234567".to_owned()),
            ),
            ("NCPUS".to_owned(), AllowedTypes::Integer(8)),
            (
                "Partition".to_owned(),
                AllowedTypes::String("partition2".to_owned()),
            ),
        ]);

        let component_config = ComponentConfig {
            name: "NCPUS".to_owned(),
            key: "NCPUS".to_owned(),
            key_type: ParsableType::Integer,
            key_allow_empty: false,
            default_value: None,
            scores: vec![
                ScoreConfig {
                    name: "Score1".to_owned(),
                    value: 1.0,
                    only_if: None,
                },
                ScoreConfig {
                    name: "Score2".to_owned(),
                    value: 2.0,
                    only_if: Some(OnlyIf {
                        key: "Partition".to_owned(),
                        matches: ".*1".to_owned(),
                    }),
                },
            ],
            only_if: None,
        };

        let component_scores_1 = construct_component_scores(&job_1, &component_config);
        let component_scores_2 = construct_component_scores(&job_2, &component_config);

        let expected_1 = vec![
            Score {
                name: ValidName::parse("Score1".to_owned()).unwrap(),
                value: ValidValue::parse(1.0).unwrap(),
            },
            Score {
                name: ValidName::parse("Score2".to_owned()).unwrap(),
                value: ValidValue::parse(2.0).unwrap(),
            },
        ];

        let expected_2 = vec![Score {
            name: ValidName::parse("Score1".to_owned()).unwrap(),
            value: ValidValue::parse(1.0).unwrap(),
        }];

        assert_eq!(component_scores_1, expected_1);
        assert_eq!(component_scores_2, expected_2);
    }
}
