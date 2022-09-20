// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anyhow::Context;
use auditor::{
    client::AuditorClient,
    constants::FORBIDDEN_CHARACTERS,
    domain::{Component, RecordAdd, Score},
    telemetry::{get_subscriber, init_subscriber},
};
use bytes::BytesMut;
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, Utc};
use configuration_server::{get_configuration, AllowedTypes, Settings};
use itertools::Itertools;
use message::Message;
use regex::Regex;
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    process::Command,
    sync::{mpsc, oneshot, Mutex},
};
use uuid::Uuid;

use crate::configuration_server::ParsableType;

mod configuration_server;
mod message;

const NAME: &str = "AUDITOR-slurm-epilog-collector";

use once_cell::sync::Lazy;

static CONFIG: Lazy<Settings> =
    Lazy::new(|| get_configuration().expect("Failed loading configuration"));
static KEYS: Lazy<Vec<(String, ParsableType)>> = Lazy::new(|| CONFIG.get_keys());

type Job = HashMap<String, AllowedTypes>;
type TransmitChannel = mpsc::Sender<(u64, Responder)>;
type ReceiveChannel = mpsc::Receiver<(u64, Responder)>;

type Responder = oneshot::Sender<()>;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = get_subscriber(NAME.into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let run_id = Uuid::new_v4();
    let span = tracing::info_span!(
        "Running slurm epilog collector server",
        %run_id,
    );
    let _span_guard = span.enter();

    let addr = format!("{}:{}", CONFIG.collector_addr, CONFIG.collector_port);

    tracing::info!("Starting  {} on {}", NAME, addr);

    let listener = TcpListener::bind(&addr).await?;

    tracing::debug!("Listening on {}", addr);

    let (tx, rx) = mpsc::channel(1024);

    let _manager = Manager::new(rx);

    loop {
        let tx = tx.clone();
        let blah = listener.accept().await;
        match blah {
            // match listener.accept().await {
            Ok((socket, _)) => {
                tracing::debug!("socket thingy");
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket, tx).await {
                        tracing::error!("Failure during handling of the conection: {}", e);
                    }
                });
                tracing::debug!("THREAD SPAWNED!!!!");
            }
            Err(e) => tracing::error!("Accepting socket failed: {}", e),
        }
    }
}

pub struct QueueProcessor {
    queue_processor: tokio::task::JoinHandle<Result<(), anyhow::Error>>,
}

impl QueueProcessor {
    #[tracing::instrument(name = "Starting queue processor")]
    pub fn new(job_queue: Arc<Mutex<VecDeque<u64>>>) -> QueueProcessor {
        let queue_processor = tokio::spawn(async move {
            let client = AuditorClient::new(&CONFIG.addr, CONFIG.port)?;
            loop {
                // tracing::debug!("Locking queue.");
                let job_id = { job_queue.lock().await.pop_front() };

                if let Some(job_id) = job_id {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    match get_slurm_job_info(job_id).await {
                        Ok(job) => {
                            tracing::debug!(?job, "Acquired SLURM job info");
                            let record = RecordAdd::new(
                                format!("{}-{}", make_string_valid(&CONFIG.record_prefix), job_id),
                                make_string_valid(&CONFIG.site_id),
                                make_string_valid(&job["User"].extract_string()?),
                                make_string_valid(&job["Group"].extract_string()?),
                                construct_components(&CONFIG, &job),
                                job["Start"].extract_datetime()?,
                            )
                            .expect("Could not construct record")
                            .with_stop_time(job["End"].extract_datetime()?);

                            tracing::debug!(?record, "Constructed record.");

                            tracing::info!("Sending record to AUDITOR instance.");
                            if let Err(e) = client.add(&record).await {
                                tracing::error!("Could not send record to Auditor: {:?}", e);
                                // todo: requeue
                            }
                            tracing::debug!("DONE Sending record to AUDITOR instance.");
                        }
                        Err(e) => {
                            tracing::error!(
                                "Could not obtain job info for job {}: {:?}",
                                job_id,
                                e
                            );
                        }
                    };
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        });
        QueueProcessor { queue_processor }
    }

    #[tracing::instrument(name = "Stopping queue processor", skip(self))]
    pub async fn stop(self) -> Result<(), anyhow::Error> {
        self.queue_processor.await?
    }
}

pub struct Manager {
    manager: tokio::task::JoinHandle<Result<(), anyhow::Error>>,
    queue_processor: QueueProcessor,
}

impl Manager {
    #[tracing::instrument(name = "Starting manager", skip(rx))]
    pub fn new(mut rx: ReceiveChannel) -> Manager {
        let job_queue = Arc::new(Mutex::new(VecDeque::new()));
        let queue_processor = QueueProcessor::new(job_queue.clone());
        let manager = tokio::spawn(async move {
            // Start receiving messages
            while let Some((job_id, responder)) = rx.recv().await {
                tracing::debug!("Received message with job id: {}", job_id);
                {
                    tracing::debug!("Acquiring lock on job queue for pushing job id: {}", job_id);
                    // let mut jq = job_queue.lock().await;
                    job_queue.lock().await.push_back(job_id);
                    // jq.push_back(job_id);
                }
                let _ = responder.send(());
            }
            Ok::<(), anyhow::Error>(())
        });

        Manager {
            manager,
            queue_processor,
        }
    }

    #[tracing::instrument(name = "Stopping manager", skip(self))]
    pub async fn stop(self) -> Result<(), anyhow::Error> {
        self.queue_processor.stop().await?;
        self.manager.await?
    }
}

#[tracing::instrument(name = "Handling new connection", skip(stream, tx))]
async fn handle_connection(
    mut stream: TcpStream,
    tx: TransmitChannel,
) -> Result<(), anyhow::Error> {
    tracing::debug!("New connection");

    let mut buffer = [0; 1024];

    let len = stream
        .read(&mut buffer)
        .await
        .context("Failed to read data from stream into buffer.")?;

    let message = Message::unpack(&BytesMut::from(&buffer[..len]))
        .context("Failed to deserialize message.")?;
    tracing::debug!("Received message: {:?}", message);

    let response = match message {
        Message::JobInfo { job_id } => {
            tracing::info!("Received job id {}", job_id);

            let (resp_tx, resp_rx) = oneshot::channel();

            tracing::debug!("Sending job_id to manager.");
            tx.send((job_id, resp_tx)).await?;

            tracing::debug!("Awaiting response from manager.");
            match resp_rx.await {
                Ok(_) => {
                    tracing::debug!("Received response from manager.");
                    Message::Ok
                }
                Err(e) => {
                    tracing::error!("Error when adding job id to queue: {:?}", e);
                    Message::Error {
                        msg: "something went wrong".to_string(),
                    }
                }
            }
        }
        msg => {
            tracing::warn!("Received unacceptable message: {:?}", msg);
            Message::Error {
                msg: "Message not acceptable".to_string(),
            }
        }
    };

    let _ = stream.write_all(&response.pack()).await;
    let _ = stream.flush();
    Ok(())
}

#[tracing::instrument(name = "Getting Slurm job info via sacct")]
async fn get_slurm_job_info(job_id: u64) -> Result<Job, anyhow::Error> {
    let mut keys = KEYS.clone();
    keys.push(("Start".to_owned(), ParsableType::DateTime));
    keys.push(("End".to_owned(), ParsableType::DateTime));
    keys.push(("Group".to_owned(), ParsableType::String));
    keys.push(("User".to_owned(), ParsableType::String));

    let cmd_out = Command::new("/usr/bin/sacct")
        .arg("-a")
        .arg("-j")
        .arg(job_id.to_string())
        .arg("--format")
        .arg(keys.iter().map(|k| k.0.clone()).join(","))
        .arg("--noconvert")
        .arg("--noheader")
        .arg("-P")
        .output()
        .await?
        .stdout;

    let cmd_test = Command::new("/usr/bin/sacct")
        .arg("-e")
        .output()
        .await?
        .stdout;

    println!("keys: {:?}", keys);
    println!("cmdout: {:?}", std::str::from_utf8(&cmd_out)?);
    println!("cmdtest: {:?}", std::str::from_utf8(&cmd_test)?);

    let lines = std::str::from_utf8(&cmd_out)?
        .lines()
        .map(|l| {
            println!("line: {}", l);
            l.split('|').map(|s| s.to_owned()).collect::<Vec<String>>()
        })
        .collect::<Vec<_>>();

    println!("lines: {:?}", lines);

    let mut merged_lines: Vec<String> = vec![String::new(); keys.len()];
    for (j, merged_line) in merged_lines.iter_mut().enumerate() {
        for (i, line) in lines.iter().enumerate() {
            println!("i: {}, j: {}, lines: {}", i, j, line[j]);
            if !line[j].is_empty() {
                *merged_line = line[j].clone();
            }
        }
    }

    // merged_lines[0] = "part1".to_string();
    // merged_lines[1] = "1".to_string();
    // merged_lines[2] = "00:00.002".to_string();
    // merged_lines[3] = "00:00.004".to_string();
    // merged_lines[4] = "10M".to_string();
    // merged_lines[5] = "2022-09-12T08:00:00".to_string();
    // merged_lines[6] = "2022-09-12T08:00:00".to_string();
    // merged_lines[7] = "root".to_string();
    // merged_lines[8] = "root".to_string();

    println!("merged_lines: {:?}", merged_lines);

    Ok(merged_lines
        .iter()
        .zip(keys.into_iter())
        .map(|(v, k)| {
            (
                k.0,
                k.1.parse(v)
                    .unwrap_or_else(|_| panic!("Error during parsing")),
            )
        })
        .collect())
}

#[tracing::instrument(name = "Parsing Slurm timestamp", level = "debug")]
fn parse_slurm_timestamp<T: AsRef<str> + std::fmt::Debug>(
    timestamp: T,
) -> Result<DateTime<Utc>, anyhow::Error> {
    let local_offset = Local::now().offset().local_minus_utc();
    Ok(DateTime::<Utc>::from(DateTime::<Local>::from_local(
        NaiveDateTime::parse_from_str(timestamp.as_ref(), "%Y-%m-%dT%H:%M:%S")?,
        FixedOffset::east(local_offset),
    )))
}

#[tracing::instrument(name = "Remove forbidden characters from string", level = "debug")]
fn make_string_valid<T: AsRef<str> + fmt::Debug>(input: T) -> String {
    input.as_ref().replace(&FORBIDDEN_CHARACTERS[..], "")
}

#[tracing::instrument(
    name = "Construct components from job info and configuration",
    level = "debug"
)]
fn construct_components(config: &Settings, job: &Job) -> Vec<Component> {
    config
        .components
        .iter()
        .cloned()
        .filter(|c| {
            c.only_if.is_none() || {
                let only_if = c.only_if.as_ref().unwrap();
                let re = Regex::new(&only_if.matches)
                    .unwrap_or_else(|_| panic!("Invalid regex expression: {}", &only_if.matches));
                re.is_match(&job[&only_if.key].extract_string().unwrap_or_else(|_| {
                    panic!("Key is expectedto be a string: {:?}", job[&only_if.key])
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
                        Score::new(s.name.clone(), s.factor)
                            .unwrap_or_else(|_| panic!("Cannot construct score from {:?}", s))
                    })
                    .collect(),
            )
        })
        .collect()
}
