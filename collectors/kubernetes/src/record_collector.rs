/// Module to read general pod info. This could be from the Kubernetes API or a
/// Prometheus instance scraping from a kube-state-metrics server.
use tokio::sync::broadcast;

mod kapi_collector;
use crate::CONFIG;
pub use kapi_collector::KapiCollector;

use crate::database::Database;
use auditor::domain::RecordAdd;

use chrono::{DateTime, Utc};

pub trait RecordCollector {
    //async fn list_records(&self) -> anyhow::Result<Vec<RecordAdd>>;
    fn list_records(
        &self,
        lastcheck: &Option<DateTime<Utc>>,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<RecordAdd>>> + Send;
}

/// Use `collector` to collect records and push them to channel `record_tx`.
#[tracing::instrument(name = "Start record collector", skip_all)]
pub fn run_record_collector<C>(
    collector: C,
    //record_tx: mpsc::Sender<(u16, RecordAdd)>,
    database: Database,
    shutdown_tx: broadcast::Sender<()>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> anyhow::Result<()>
where
    C: RecordCollector + Send + 'static,
{
    let _interval: std::time::Duration = CONFIG.get().unwrap().collect_interval.to_std()?;
    let earliest_datetime: DateTime<Utc> = CONFIG.get().unwrap().earliest_datetime.into();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(_interval);
        loop {
            tokio::select! {
                _ = interval.tick() => {},
                _ = shutdown_rx.recv() => {
                    tracing::info!("Kubernetes monitor received shutdown signal. Shutting down.");
                    break
                }
            }
            let lastcheck = match database.get_lastcheck().await {
                Ok(last) => std::cmp::max(last, Some(earliest_datetime)),
                Err(e) => {
                    tracing::error!("Record collector failed reading from db: {}", e);
                    shutdown_tx.send(()).expect("Shutdown channel lost");
                    break;
                }
            };
            let now = Utc::now();
            tokio::select! {
                records = collector.list_records(&lastcheck) => {
                    if let Err(e) = records {
                        tracing::error!("Cannot retrieve from Kubernetes: {}", e);
                        continue
                    };
                    if let Err(e) = database.insert_many(&records.unwrap()).await {
                        tracing::error!("{}", e);
                        shutdown_tx.send(()).expect("Shutdown channel lost");
                        break
                    };
                    if let Err(e) = database.set_lastcheck(now).await {
                        tracing::error!("{}", e);
                        shutdown_tx.send(()).expect("Shutdown channel lost");
                        break
                    }
                },
                _ = shutdown_rx.recv() => {
                    tracing::info!("Kubernetes monitor received shutdown signal. Shutting down.");
                    break
                }
            }
        }
        // Precaution that ensures that the tx is moved into the async block
        drop(shutdown_tx);
    });
    Ok(())
}
