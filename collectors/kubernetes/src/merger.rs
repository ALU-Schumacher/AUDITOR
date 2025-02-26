use std::error::Error;
use std::fmt::{self, Display};
use std::time::Duration;

use crate::{
    CONFIG,
    constants::{COMPONENT_CPU, COMPONENT_MEM, KEY_NAMESPACE, KEY_PODNAME},
    database::Database,
};
use auditor::domain::{Component, RecordAdd, ValidMeta, ValidName};
use auditor_client::{AuditorClient as AClient, ClientError};

use anyhow::Context;
use chrono::{DateTime, Utc};
use prometheus_http_query::Client as PClient;
use prometheus_http_query::error::Error as PError;
use prometheus_http_query::error::PrometheusErrorType;
use reqwest::ClientBuilder;
use tokio::sync::broadcast;

#[derive(Debug)]
enum MergeError {
    /// A [`RecordAdd`] is considered to be well-formed if:
    /// - `stop_time` and `meta` are both `Some`
    /// - `meta` contains the keys defined by [`KEY_NAMESPACE`] and
    ///   [`KEY_PODNAME`]
    /// - Both corresponding value vectors contain one entry
    RecordMalformed,
    NoConnection,
    Incomplete,
    // The string is printed
    #[allow(dead_code)]
    Critical(String),
}

impl Display for MergeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for MergeError {}

// TODO: errors not documented. needs verification
/// Check if a `prometheus_http_query::error::Error` is some kind of connection error.
fn is_connection(e: &prometheus_http_query::error::Error) -> bool {
    match e {
        PError::Client(e) => {
            if let Some(e) = e.inner() {
                if e.is_timeout() || e.is_connect() {
                    return true;
                }
            }
        }
        PError::Prometheus(e) => match e.error_type() {
            PrometheusErrorType::Timeout | PrometheusErrorType::Unavailable => return true,
            _ => return false,
        },
        _ => return false,
    };
    false
}

pub fn build_pclient(url: &str, timeout: Duration) -> anyhow::Result<PClient> {
    let client = ClientBuilder::new().timeout(timeout).build()?;
    Ok(PClient::from(client, url)?)
}

/// Starts the merger task. The records received through `rx` are merged with the
/// appropriate resource metrics from Prometheus.
/// The number received along the records is the retry number.
#[tracing::instrument(name = "Start merger task", skip_all)]
pub fn run_merger(
    database: Database,
    shutdown_tx: broadcast::Sender<()>,
    shutdown_rx: broadcast::Receiver<()>,
    aclient: AClient,
    pclient: PClient,
) -> anyhow::Result<()> {
    let interval: std::time::Duration = CONFIG.get().unwrap().merge_interval.to_std()?;

    tokio::spawn(process_queue(
        //rx,
        database,
        interval,
        shutdown_tx,
        shutdown_rx,
        aclient,
        pclient,
        //backlog,
    ));

    Ok(())
}

fn component_exists(components: &[Component], name: &ValidName) -> bool {
    components.iter().any(|c| &c.name == name)
}

/// Retrieve the value for key `name` if and only if it exists and the
/// associated vector has exactly one entry
#[tracing::instrument(name = "Get meta entry", level = "trace", skip(meta))]
fn get_meta<'a>(meta: &'a ValidMeta, name: &ValidName) -> Option<&'a ValidName> {
    meta.0
        .get(name)
        .and_then(|v| if v.len() > 1 { None } else { v.first() })
}

/// Execute the `query` at time `stoptime` and return the result.
/// Assumes that the query will yield a vector with one entry.
#[tracing::instrument(name = "Retrieve resource metrics", level = "trace", skip(client))]
async fn obtain_metric(
    client: &PClient,
    query: &str,
    stoptime: &DateTime<Utc>,
) -> Result<i64, MergeError> {
    let metric = match client.query(query).at(stoptime.timestamp()).get().await {
        Ok(res) => res
            .into_inner()
            .0 // data
            .into_vector()
            .map_err(|data| {
                MergeError::Critical(format!("Bad result from Prometheus: {:?}", data))
            })?,
        Err(e) => {
            if is_connection(&e) {
                return Err(MergeError::NoConnection);
            } else {
                return Err(MergeError::Critical(format!("{:?}", e)));
            }
        }
    };
    if metric.len() > 1 {
        return Err(MergeError::Critical(format!(
            "Retrieved too many series for query {}",
            query
        )));
    };
    metric
        .first()
        .map(|v| v.sample().value().round() as i64)
        .ok_or(MergeError::Incomplete)
}

/// Takes a `RecordAdd` and tries to fill it with resource metrics from Prometheus,
/// obtained through `client`.
///
/// # Errors:
/// - [`MergeError::RecordMalformed`] if `rec` is malformed
/// - [`MergeError::NoConnection`] if there are connection problems to Prometheus
/// - [`MergeError::Incomplete`] if Prometheus was reached but any of the queries
///   returned an empty result
/// - [`MergeError::Critical`] on any other error
#[tracing::instrument(
    name = "Complete Record",
    level = "debug",
    skip_all,
    fields(record_id = %rec.record_id),
)]
async fn fill_record(rec: &mut RecordAdd, client: &PClient) -> Result<(), MergeError> {
    // Stop time and duration
    // TODO: Timezones
    let starttime = rec.start_time;
    let stoptime = rec.stop_time.ok_or(MergeError::RecordMalformed)?;
    let duration = (stoptime - starttime).num_seconds() + 1;

    // Names of namespace and pod
    let meta = rec.meta.as_ref().ok_or(MergeError::RecordMalformed)?;
    let namespace = get_meta(meta, &KEY_NAMESPACE).ok_or(MergeError::RecordMalformed)?;
    let pod = get_meta(meta, &KEY_PODNAME).ok_or(MergeError::RecordMalformed)?;

    // Queries for cpu and mem
    let labels = format!(r#"namespace="{namespace}",pod="{pod}""#);
    // TODO: restarted pods: should be done with max_over_time/increase?
    let cpu_query = format!(
        r#"sum by (namespace,pod) (
        max_over_time(increase(
        pod_cpu_usage_seconds_total{{{0}}}[{1}s])[{1}s:]
        ))"#,
        labels, duration
    );
    let mem_query = format!(
        r#"sum by (namespace,pod) (
        max_over_time(pod_memory_working_set_bytes{{{}}}[{}s]))"#,
        labels, duration
    );

    // Obtain CPU
    if !component_exists(&rec.components, &COMPONENT_CPU) {
        let cpu = obtain_metric(client, &cpu_query, &stoptime).await?;
        let component = Component::new(&COMPONENT_CPU, cpu)
            .context("Invalid component")
            .map_err(|e| MergeError::Critical(e.to_string()))?;
        rec.components.push(component);
    };

    // Obtain Mem
    if !component_exists(&rec.components, &COMPONENT_MEM) {
        let mem = obtain_metric(client, &mem_query, &stoptime).await?;
        let component = Component::new(&COMPONENT_MEM, mem)
            .context("Invalid component")
            .map_err(|e| MergeError::Critical(e.to_string()))?;
        rec.components.push(component);
    };

    // Return
    if component_exists(&rec.components, &COMPONENT_CPU)
        && component_exists(&rec.components, &COMPONENT_MEM)
    {
        Ok(())
    } else {
        Err(MergeError::Incomplete)
    }
}

/// Tries to complete all mergeable records using Prometheus.
///
/// Errors: On failed DB operations and MergeError::Critical
#[tracing::instrument(name = "Complete mergeable Records", level = "debug", skip_all)]
async fn merge(database: &Database, pclient: &PClient) -> anyhow::Result<()> {
    let records = database
        .get_mergequeue()
        .await
        .context("Failed reading from queue")?;
    for mut r in records {
        match fill_record(&mut r, pclient).await {
            Ok(_) => database
                .replace_complete(&r)
                .await
                .context("Failed DB update")?,
            Err(MergeError::NoConnection) => {
                tracing::warn!("Can't reach Prometheus. Will retry...");
            }
            Err(MergeError::Incomplete) => {
                tracing::warn!("Record {} still incomplete", r.record_id);
                database
                    .replace_incomplete(&r)
                    .await
                    .context("Failed DB update")?;
            }
            res @ Err(_) => return res.context("Failed merging metrics"),
        }
    }
    Ok(())
}

/// Tries to send all records that are complete or have exceeded their retries.
///
/// Errors: Only on DB fails
#[tracing::instrument(name = "Send Records to AUDITOR", level = "debug", skip_all)]
async fn send(database: &Database, aclient: &AClient) -> anyhow::Result<()> {
    let incomplete = database
        .get_incomplete()
        .await
        .context("Failed reading from queue")?;
    let mut records = database
        .get_complete()
        .await
        .context("Failed reading from queue")?;
    let ids: Vec<_> = incomplete.iter().map(|r| r.record_id.as_ref()).collect();
    if !ids.is_empty() {
        tracing::warn!("Sending incomplete records: {:?}", ids);
    };
    records.extend(incomplete);
    for r in records {
        match aclient.add(&r).await {
            Ok(()) => {}
            Err(ClientError::RecordExists) => {
                tracing::warn!("Record {} already exists in AUDITOR", r.record_id.as_ref())
            }
            Err(e) => {
                tracing::error!("While sending to AUDITOR: {}", e);
                continue;
            }
        };
        database
            .delete(r.record_id.as_ref())
            .await
            .context("Failed deleting from DB")?;
    }
    Ok(())
}

/// Retrieves `RecordAdd`s through `rx` and calls [`fill_record`] on them.
///
/// The following strategy is applied:
/// - If the record is filled successfully, it is send to AUDITOR.
/// - `MergeError::Critical` and `MergeError::RecordMalformed` imply programming errors. The
///   program is shut down through `shutdown_tx`.
/// - On `MergeError::Incomplete`: If there have been too
///   many retries, the record is sent incompletely. Else it is put into the
///   `backlog`.
/// - On `MergeError::NoConnection`: Push to backlog, does not count as retry
async fn process_queue(
    database: Database,
    interval: Duration,
    shutdown_tx: broadcast::Sender<()>,
    mut shutdown_rx: broadcast::Receiver<()>,
    aclient: AClient,
    pclient: PClient,
) {
    let mut sleeper = tokio::time::interval(interval);
    loop {
        tokio::select! {
            _ = sleeper.tick() => {},
            _ = shutdown_rx.recv() => {
                tracing::info!("process_queue received shutdown signal. Shutting down.");
                break
            }
        };

        // Update Records
        tokio::select! {
            res = merge(&database, &pclient) => if let Err(e) = res
                .context("Failed merge operation")
            {
                tracing::error!(%e);
                shutdown_tx.send(()).expect("Shutdown channel lost");
                break
            },
            _ = shutdown_rx.recv() => {
                tracing::info!("process_queue received shutdown signal. Shutting down.");
                break
            }
        };

        // Send Records
        tokio::select! {
            res = send(&database, &aclient) => if let Err(e) = res
                .context("Failed merge operation")
            {
                tracing::error!(%e);
                shutdown_tx.send(()).expect("Shutdown channel lost");
                break
            },
            _ = shutdown_rx.recv() => {
                tracing::info!("process_queue received shutdown signal. Shutting down.");
                break
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auditor::domain::ValidName;
    use std::collections::HashMap;
    use std::time::Duration;
    use wiremock::matchers::{method, path, query_param_contains};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_component_exists() {
        let components = vec![Component::new("cpu", 10).unwrap()];
        assert!(component_exists(
            &components,
            &ValidName::parse("cpu".to_owned()).unwrap()
        ));
        assert!(!component_exists(
            &components,
            &ValidName::parse("pu".to_owned()).unwrap()
        ));
    }

    #[test]
    fn test_get_meta() {
        let test = ValidName::parse("test".to_owned()).unwrap();
        let empty = ValidName::parse("empty".to_owned()).unwrap();
        let one = ValidName::parse("one".to_owned()).unwrap();
        let two = ValidName::parse("two".to_owned()).unwrap();
        let hm: HashMap<_, _> = [
            (empty.clone(), Vec::new()),
            (one.clone(), vec![one.clone()]),
            (two.clone(), vec![one.clone(), two.clone()]),
        ]
        .into();
        let meta = ValidMeta(hm);
        assert!(get_meta(&meta, &test).is_none());
        assert!(get_meta(&meta, &empty).is_none());
        assert!(get_meta(&meta, &two).is_none());
        assert_eq!(get_meta(&meta, &one).unwrap().as_ref(), one.as_ref());
    }

    #[tokio::test]
    async fn test_obtain_metric() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = PClient::try_from(uri).unwrap();

        let query = r#"sum by (namespace,pod) (
            max_over_time(pod_memory_working_set_bytes{{app==test}}[60s]))"#
            .to_string();
        let response = r#"
        {
          "status": "success",
          "data": {
            "resultType": "vector",
            "result": [
              {
                "metric": {
                  "namespace": "auditor",
                  "pod": "prometheus-auditor-prometheus-78d6c69bb6-dk8rq"
                },
                "value": [
                  1714910734.510,
                  "122986496"
                ]
              }
            ]
          }
        }
        "#;

        Mock::given(method("GET"))
            .and(path("/api/v1/query"))
            .and(query_param_contains("query", query.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_raw(response, "application/json"))
            .expect(1)
            .mount(&mock_server)
            .await;
        let response = obtain_metric(&client, &query, &DateTime::<Utc>::default())
            .await
            .unwrap();
        assert_eq!(response, 122986496);
    }

    #[tokio::test]
    async fn test_obtain_metric_empty_result() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = PClient::try_from(uri).unwrap();

        let query = r#"sum by (namespace,pod) (
            max_over_time(pod_memory_working_set_bytes{{app==test}}[60s]))"#
            .to_string();
        let response = r#"
        {
          "status": "success",
          "data": {
            "resultType": "vector",
            "result": []
          }
        }
        "#;

        Mock::given(method("GET"))
            .and(path("/api/v1/query"))
            .and(query_param_contains("query", query.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_raw(response, "application/json"))
            .expect(1)
            .mount(&mock_server)
            .await;
        let response = obtain_metric(&client, &query, &DateTime::<Utc>::default()).await;
        assert!(matches!(response.unwrap_err(), MergeError::Incomplete));
    }

    #[tokio::test]
    async fn test_obtain_metric_bad_result() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = PClient::try_from(uri).unwrap();

        let query = r#"sum by (namespace,pod) (
            max_over_time(pod_memory_working_set_bytes{{app==test}}[60s]))"#
            .to_string();
        let response = r#"
        {
          "status": "success",
          "data": {
            "resultType": "matrix",
            "result": []
          }
        }
        "#;

        Mock::given(method("GET"))
            .and(path("/api/v1/query"))
            .and(query_param_contains("query", query.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_raw(response, "application/json"))
            .expect(1)
            .mount(&mock_server)
            .await;
        let response = obtain_metric(&client, &query, &DateTime::<Utc>::default()).await;
        assert!(matches!(response.unwrap_err(), MergeError::Critical(_)));
    }

    #[tokio::test]
    async fn test_obtain_metric_too_many_points() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = PClient::try_from(uri).unwrap();

        let query = r#"sum by (namespace,pod) (
            max_over_time(pod_memory_working_set_bytes{{app==test}}[60s]))"#
            .to_string();
        let response = r#"
        {
          "status": "success",
          "data": {
            "resultType": "vector",
            "result": [
              {
                "metric": {
                  "namespace": "auditor",
                  "pod": "prometheus-auditor-prometheus-78d6c69bb6-dk8rq"
                },
                "value": [
                  1714910734.510,
                  "122986496"
                ]
              },
              {
                "metric": {
                  "namespace": "auditor",
                  "pod": "prometheus-auditor-prometheus"
                },
                "value": [
                  1714910734.510,
                  "122986496"
                ]
              }
            ]
          }
        }
        "#;

        Mock::given(method("GET"))
            .and(path("/api/v1/query"))
            .and(query_param_contains("query", query.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_raw(response, "application/json"))
            .expect(1)
            .mount(&mock_server)
            .await;
        let response = obtain_metric(&client, &query, &DateTime::<Utc>::default()).await;
        assert!(matches!(response.unwrap_err(), MergeError::Critical(_)));
    }

    #[tokio::test]
    async fn test_obtain_metric_timeout() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        let client = build_pclient(&uri, Duration::from_millis(100)).unwrap();

        let query = r#"sum by (namespace,pod) (
            max_over_time(pod_memory_working_set_bytes{{app==test}}[60s]))"#
            .to_string();
        let response = r#"
        {
          "status": "success",
          "data": {
            "resultType": "vector",
            "result": []
          }
        }
        "#;

        Mock::given(method("GET"))
            .and(path("/api/v1/query"))
            .and(query_param_contains("query", query.clone()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(response, "application/json")
                    .set_delay(Duration::from_secs(3600)),
            )
            .expect(1)
            .mount(&mock_server)
            .await;
        let response = obtain_metric(&client, &query, &DateTime::<Utc>::default()).await;
        assert!(matches!(response.unwrap_err(), MergeError::NoConnection));
    }
}
