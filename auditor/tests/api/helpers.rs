use auditor::configuration::{get_configuration, DatabaseSettings};
use auditor::metrics::DatabaseMetricsWatcher;
use auditor::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use tracing_subscriber::filter::LevelFilter;
use urlencoding::encode;
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
            .post(&format!("{}/record", &self.address))
            .header("Content-Type", "application/json")
            .json(record)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn bulk_insert<T: serde::Serialize>(&self, record: &T) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/records", &self.address))
            .header("Content-Type", "application/json")
            .json(record)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_records(&self) -> reqwest::Response {
        reqwest::Client::new()
            .get(&format!("{}/records", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_started_since_records<T: AsRef<str>>(
        &self,
        timestamp: T,
    ) -> reqwest::Response {
        let timestamp_str = timestamp.as_ref();
        let encoded_since = encode(timestamp_str);
        reqwest::Client::new()
            .get(&format!(
                "{}/records?start_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_stopped_since_records<T: AsRef<str>>(
        &self,
        timestamp: T,
    ) -> reqwest::Response {
        let timestamp_str = timestamp.as_ref();
        let encoded_since = encode(timestamp_str);
        reqwest::Client::new()
            .get(&format!(
                "{}/records?stop_time[gte]={}",
                &self.address, encoded_since
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn advanced_queries<T: AsRef<str> + std::fmt::Display>(
        &self,
        query_string: T,
    ) -> reqwest::Response {
        reqwest::Client::new()
            .get(&format!("{}/records?{}", &self.address, query_string))
            .send()
            .await
            .expect("Failed to execute queries.")
    }

    pub async fn get_single_record<T: AsRef<str> + std::fmt::Display>(
        &self,
        record_id: T,
    ) -> reqwest::Response {
        reqwest::Client::new()
            .get(&format!("{}/record/{}", &self.address, record_id))
            .send()
            .await
            .expect("Failed to execute queries.")
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
