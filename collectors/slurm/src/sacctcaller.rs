// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::{collections::HashMap, fmt};

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
    configuration::{AllowedTypes, ParsableType},
    database::Database,
    shutdown::Shutdown,
    CONFIG, END, GROUP, JOBID, KEYS, START, USER,
};

type Job = HashMap<String, AllowedTypes>;

static BATCH_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^[0-9_]+\.batch$"#)
        .expect("Could not construct essential Regex for matching job ids.")
});

static SUB_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^[0-9]+\.[0-9]*$"#)
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

    let cmd_out = Command::new("/usr/bin/sacct")
        .arg("-a")
        .arg("--format")
        .arg(KEYS.iter().map(|k| k.0.clone()).join(","))
        .arg("--noconvert")
        .arg("--noheader")
        .arg("-S")
        .arg(format!("{}", lastcheck.format("%Y-%m-%dT%H:%M:%S")))
        .arg("-E")
        .arg("now")
        .arg("-s")
        .arg(CONFIG.job_status.join(","))
        .arg("-P")
        .output()
        .await?;

    let sacct_rows = std::str::from_utf8(&cmd_out.stdout)?
        .lines()
        .map(|l| {
            KEYS.iter()
                .cloned()
                .zip(l.split('|').map(|s| s.to_owned()))
                // Occasionally fields are empty by design. filter those out to avoid
                // problems later on when parsing.
                .filter(|(_, v)| !v.is_empty())
                .map(|((k, pt), v)| {
                    let v = match pt.parse(&v) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!(
                                "Parsing '{}' (key: {}) as {:?} failed: {:?}. This may or may not be a problem. It probably is.",
                                v,
                                k,
                                pt,
                                e
                            );
                            None
                        }
                    };
                    (k, v)
                })
                .collect::<HashMap<String, Option<AllowedTypes>>>()
        })
        .map(|hm| (hm[JOBID].as_ref().unwrap().extract_string().unwrap(), hm))
        .collect::<HashMap<String, HashMap<String,Option<AllowedTypes>>>>();

    let records = sacct_rows
        .keys()
        .filter(|k| !BATCH_REGEX.is_match(k))
        .filter(|k| !SUB_REGEX.is_match(k))
        .map(|id| -> Result<HashMap<String, AllowedTypes>> {
            let map1 = sacct_rows.get(id).ok_or(eyre!("Cannot get map1"))?;
            let map2 = sacct_rows.get(&format!("{id}.batch"));
            Ok(KEYS.iter()
                .cloned()
                .filter_map(|(k, _)| {
                    let val = match map1.get(&k) {
                        Some(Some(v)) => Some(v.clone()),
                        _ => {
                            if let Some(map2) = map2 {
                                match map2.get(&k) {
                                    Some(Some(v)) => Some(v.clone()),
                                    _ => {
                                        tracing::error!("Something went wrong during parsing (id: {id}, key: {k})");
                                        None
                                    },
                                }
                            } else {
                                tracing::error!("Something went wrong during parsing (id: {id}, key: {k})");
                                None
                            }
                        },
                    };
                    val.map(|val| (k, val))
                })
                .collect::<HashMap<String, AllowedTypes>>())
    }).collect::<Result<Vec<HashMap<String, AllowedTypes>>>>()?
    .iter()
    .map(|map| -> Result<Option<RecordAdd>> {
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
            meta.iter().map(|m| -> Result<Vec<(String, Vec<String>)>> {
                let map = if m.key_type == ParsableType::Json {
                    if let Some(val) = map.get(&m.key) {
                        val
                            .extract_map()?
                            .iter()
                            .map(|(k, v)| -> Result<(String, Vec<String>)> {
                                    Ok(
                                        (
                                            make_string_valid(k.extract_string()?),
                                            vec![make_string_valid(v.extract_string()?)]
                                        )
                                    )
                                }
                            )
                            .collect::<Result<Vec<(_, _)>>>()?
                    } else {
                        vec![]
                    }
                } else {
                    vec![(m.name.clone(), vec![make_string_valid(map[&m.key].extract_as_string()?)])]
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
        meta.insert("user_id".to_string(), vec![make_string_valid(map[USER].extract_string()?)]);
        meta.insert("group_id".to_string(), vec![make_string_valid(map[GROUP].extract_string()?)]);

        Ok(Some(
           RecordAdd::new(
               record_id,
               meta,
               construct_components(map),
               map[START].extract_datetime()?
           )
           .expect("Could not construct record")
           .with_stop_time(map[END].extract_datetime()?)
        ))
    })
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
            DateTime::<Local>::from_utc(
                ts.naive_utc(),
                FixedOffset::east_opt(local_offset).unwrap(),
            ),
            rid,
        )
    };

    database.set_lastcheck(rid, nextcheck).await?;

    Ok(records)
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
    level = "debug"
)]
fn construct_components(job: &Job) -> Vec<Component> {
    CONFIG
        .components
        .iter()
        .cloned()
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
        .map(|c| {
            Component::new(
                make_string_valid(c.name),
                job[&c.key].extract_i64().unwrap_or_else(|_| {
                    panic!(
                        "Cannot parse key {} (value: {:?}) into i64.",
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
                    .collect(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_job_ids() {
        assert!(BATCH_REGEX.is_match("1234.batch"));
        assert!(BATCH_REGEX.is_match("1234_10.batch"));
    }
}
