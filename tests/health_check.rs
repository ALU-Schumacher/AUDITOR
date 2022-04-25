use auditor::configuration::{get_configuration, DatabaseSettings};
use auditor::domain::{Component, Record, RecordTest};
use auditor::telemetry::{get_subscriber, init_subscriber};
use fake::{Fake, Faker};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
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

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;
    let server =
        auditor::startup::run(listener, connection_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    TestApp {
        address,
        db_pool: connection_pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
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
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn add_returns_a_200_for_valid_json_data() {
    // Arange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    for _ in 0..100 {
        let body: RecordTest = Faker.fake();

        let response = client
            .post(&format!("{}/add", &app.address))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(200, response.status().as_u16());

        let saved = sqlx::query_as!(
            Record,
            r#"SELECT
           record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
           start_time, stop_time, runtime
           FROM accounting
           WHERE record_id = $1
        "#,
            body.record_id.as_ref().unwrap(),
        )
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch data.");

        assert_eq!(body, saved);
    }
}

#[tokio::test]
async fn add_returns_a_400_for_invalid_json_data() {
    // Arange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let forbidden_strings: Vec<String> = ['/', '(', ')', '"', '<', '>', '\\', '{', '}']
        .into_iter()
        .map(|s| format!("test{}test", s))
        .collect();

    for field in ["record_id", "site_id", "group_id", "user_id"] {
        for fs in forbidden_strings.iter() {
            // Act
            let mut body: RecordTest = Faker.fake();
            match field {
                "record_id" => body.record_id = Some(fs.clone()),
                "site_id" => body.site_id = Some(fs.clone()),
                "group_id" => body.group_id = Some(fs.clone()),
                "user_id" => body.user_id = Some(fs.clone()),
                _ => (),
            }

            let response = client
                .post(&format!("{}/add", &app.address))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .expect("Failed to execute request.");

            assert_eq!(400, response.status().as_u16());

            let saved = sqlx::query!(
                r#"SELECT
                   record_id, site_id, user_id, group_id,
                   components as "components: Vec<Component>",
                   start_time, stop_time, runtime
                   FROM accounting
                   WHERE record_id = $1
                "#,
                body.record_id.as_ref().unwrap(),
            )
            .fetch_all(&app.db_pool)
            .await
            .expect("Failed to fetch data.");

            assert_eq!(saved.len(), 0);
        }
    }
}

#[tokio::test]
async fn add_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let record: RecordTest = Faker.fake();

    let test_cases = vec![
        ("record_id is missing", {
            let mut r = record.clone();
            r.record_id = None;
            r
        }),
        ("site_id is missing", {
            let mut r = record.clone();
            r.site_id = None;
            r
        }),
        ("user_id is missing", {
            let mut r = record.clone();
            r.user_id = None;
            r
        }),
        ("group_id is missing", {
            let mut r = record.clone();
            r.group_id = None;
            r
        }),
        ("components is missing", {
            let mut r = record.clone();
            r.components = None;
            r
        }),
        ("start_time is missing", {
            let mut r = record.clone();
            r.start_time = None;
            r
        }),
    ];

    for (error_message, invalid_body) in test_cases {
        // Act
        let response = client
            .post(&format!("{}/add", &app.address))
            .header("Content-Type", "application/json")
            .json(&invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn update_returns_a_400_for_non_existing_record() {
    // Arange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let body: RecordTest = Faker.fake();

    let response = client
        .post(&format!("{}/update", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(400, response.status().as_u16());
}

#[tokio::test]
async fn update_returns_a_200_for_valid_form_data() {
    // Arange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    // first add a record
    let mut body: RecordTest = Faker.fake();
    body = body.with_start_time("2022-03-01T12:00:00-00:00");
    body.stop_time = None;

    let response = client
        .post(&format!("{}/add", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    // Update this record
    let body = body.with_stop_time("2022-03-01T13:00:00-00:00");

    let response = client
        .post(&format!("{}/update", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query_as!(
        Record,
        r#"SELECT
           record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
           start_time, stop_time, runtime
           FROM accounting
           WHERE record_id = $1
        "#,
        body.record_id.as_ref().unwrap()
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch data.");

    assert_eq!(saved, body);
}

#[tokio::test]
async fn get_returns_a_200_and_list_of_records() {
    // Arange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // First send a couple of records
    let mut test_cases: Vec<RecordTest> = (0..100)
        .into_iter()
        .map(|_| Faker.fake::<RecordTest>())
        .collect();

    for case in test_cases.iter() {
        let response = client
            .post(&format!("{}/add", &app.address))
            .header("Content-Type", "application/json")
            .json(&case)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(200, response.status().as_u16());
    }

    let response = client
        .get(&format!("{}/get", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(200, response.status().as_u16());

    let mut received_records = response.json::<Vec<Record>>().await.unwrap();

    // make sure they are both sorted
    test_cases.sort_by(|a, b| {
        a.record_id
            .as_ref()
            .unwrap()
            .cmp(b.record_id.as_ref().unwrap())
    });
    received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

    for (i, (record, received)) in test_cases.iter().zip(received_records.iter()).enumerate() {
        assert_eq!(
            record,
            received,
            "Check {}: Record {} and {} did not match.",
            i,
            record.record_id.as_ref().unwrap(),
            received.record_id
        );
    }
}

#[tokio::test]
async fn get_returns_a_200_and_no_records() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/get", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(200, response.status().as_u16());

    let received_records = response.json::<Vec<Record>>().await.unwrap();

    assert!(received_records.is_empty());
}

#[tokio::test]
async fn get_started_since_returns_a_200_and_list_of_records() {
    // Arange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // First send a couple of records
    let test_cases = (1..10)
        .into_iter()
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                // Giving a name which is sorted the same as the time is useful for asserting later
                .with_record_id(format!("r{}", i))
                .with_start_time(format!("2022-03-0{}T12:00:00-00:00", i))
        })
        .collect::<Vec<_>>();

    for case in test_cases.iter() {
        let response = client
            .post(&format!("{}/add", &app.address))
            .header("Content-Type", "application/json")
            .json(&case)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(200, response.status().as_u16());
    }

    // Try different start dates and receive records
    for i in 1..10 {
        let response = client
            .get(&format!(
                "{}/get/started/since/2022-03-0{}T00:00:00-00:00",
                &app.address, i
            ))
            .send()
            .await
            .expect("Failed to execute request.");
        assert_eq!(200, response.status().as_u16());

        let mut received_records = response.json::<Vec<Record>>().await.unwrap();

        // make sure they are both sorted
        received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

        for (j, (record, received)) in test_cases
            .iter()
            .skip(i - 1)
            .zip(received_records.iter())
            .enumerate()
        {
            assert_eq!(
                record,
                received,
                "Check {}|{}: Record {} and {} did not match.",
                i,
                j,
                record.record_id.as_ref().unwrap(),
                received.record_id
            );
        }
    }
}

#[tokio::test]
async fn get_started_since_returns_a_200_and_no_records() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!(
            "{}/get/started/since/2022-03-01T13:00:00-00:00",
            &app.address
        ))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(200, response.status().as_u16());

    let received_records = response.json::<Vec<Record>>().await.unwrap();

    assert!(received_records.is_empty());
}

#[tokio::test]
async fn get_stopped_since_returns_a_200_and_list_of_records() {
    // Arange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // First send a couple of records
    let test_cases = (1..10)
        .into_iter()
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                // Giving a name which is sorted the same as the time is useful for asserting later
                .with_record_id(format!("r{}", i))
                .with_stop_time(format!("2022-03-0{}T12:00:00-00:00", i))
        })
        .collect::<Vec<_>>();

    for case in test_cases.iter() {
        let response = client
            .post(&format!("{}/add", &app.address))
            .header("Content-Type", "application/json")
            .json(&case)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(200, response.status().as_u16());
    }

    // Try different start dates and receive records
    for i in 1..10 {
        let response = client
            .get(&format!(
                "{}/get/stopped/since/2022-03-0{}T00:00:00-00:00",
                &app.address, i
            ))
            .send()
            .await
            .expect("Failed to execute request.");
        assert_eq!(200, response.status().as_u16());

        let mut received_records = response.json::<Vec<Record>>().await.unwrap();

        // make sure they are both sorted
        received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

        for (j, (record, received)) in test_cases
            .iter()
            .skip(i - 1)
            .zip(received_records.iter())
            .enumerate()
        {
            assert_eq!(
                record,
                received,
                "Check {}|{}: Record {} and {} did not match.",
                i,
                j,
                record.record_id.as_ref().unwrap(),
                received.record_id
            );
        }
    }
}

#[tokio::test]
async fn get_stopped_since_returns_a_200_and_no_records() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!(
            "{}/get/stopped/since/2022-03-01T13:00:00-00:00",
            &app.address
        ))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(200, response.status().as_u16());

    let received_records = response.json::<Vec<Record>>().await.unwrap();

    assert!(received_records.is_empty());
}

#[tokio::test]
async fn get_wrong_since_returns_a_404() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!(
            "{}/get/wrong/since/2022-03-01T13:00:00-00:00",
            &app.address
        ))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(404, response.status().as_u16());
}
