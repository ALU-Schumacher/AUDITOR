use auditor::configuration::{get_configuration, DatabaseSettings};
use auditor::metrics::DatabaseMetricsWatcher;
use auditor::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use tracing_subscriber::filter::LevelFilter;
use uuid::Uuid;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = LevelFilter::INFO;
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

impl TestApp {
    pub async fn health_check(&self) -> reqwest::Response {
        reqwest::Client::new()
            .get(format!("{}/health_check", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn add_record<T: serde::Serialize>(&self, record: &T) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/add", &self.address))
            .header("Content-Type", "application/json")
            .json(record)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_records(&self) -> reqwest::Response {
        reqwest::Client::new()
            .get(&format!("{}/get", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_started_since_records<T: AsRef<str>>(
        &self,
        timestamp: T,
    ) -> reqwest::Response {
        reqwest::Client::new()
            .get(&format!(
                "{}/get/started/since/{}",
                &self.address,
                timestamp.as_ref()
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_stopped_since_records<T: AsRef<str>>(
        &self,
        timestamp: T,
    ) -> reqwest::Response {
        reqwest::Client::new()
            .get(&format!(
                "{}/get/stopped/since/{}",
                &self.address,
                timestamp.as_ref()
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;
    let db_watcher = DatabaseMetricsWatcher::new(connection_pool.clone(), &configuration).unwrap();
    let server = auditor::startup::run(listener, connection_pool.clone(), db_watcher)
        .expect("Failed to bind address");
    tokio::spawn(server);
    TestApp {
        address,
        db_pool: connection_pool,
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./../migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}
