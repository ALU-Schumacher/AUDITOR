use auditor::configuration::{get_configuration, DatabaseSettings};
use auditor::record::{Component, Record, RecordAdd, RecordUpdate};
use auditor::telemetry::{get_subscriber, init_subscriber};
use chrono::{DateTime, Utc};
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
    let mut connection = PgConnection::connect(&config.connection_string_without_db())
        .await
        .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect(&config.connection_string())
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
    let body = RecordAdd {
        record_id: "hpc-1337".into(),
        site_id: "cluster1".into(),
        user_id: "user1".into(),
        group_id: "group1".into(),
        components: vec![
            Component {
                name: "CPU".into(),
                amount: 10,
                factor: 1.3,
            },
            Component {
                name: "Memory".into(),
                amount: 120,
                factor: 1.0,
            },
        ],
        start_time: DateTime::parse_from_rfc3339("2022-03-01T12:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc),
        stop_time: Some(
            DateTime::parse_from_rfc3339("2022-03-01T13:00:00-00:00")
                .unwrap()
                .with_timezone(&Utc),
        ),
    };

    let response = client
        .post(&format!("{}/add", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!(
        r#"SELECT
           record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
           start_time, stop_time, runtime
           FROM accounting
           WHERE record_id = $1
        "#,
        "hpc-1337",
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.record_id, "hpc-1337");
    assert_eq!(saved.site_id.unwrap(), "cluster1");
    assert_eq!(saved.user_id.unwrap(), "user1");
    assert_eq!(saved.group_id.unwrap(), "group1");
    assert_eq!(saved.components.as_ref().unwrap()[0].name, "CPU");
    assert_eq!(saved.components.as_ref().unwrap()[0].amount, 10);
    assert_eq!(
        saved.components.as_ref().unwrap()[0].factor.to_ne_bytes(),
        1.3f64.to_ne_bytes()
    );
    assert_eq!(saved.components.as_ref().unwrap()[1].name, "Memory");
    assert_eq!(saved.components.as_ref().unwrap()[1].amount, 120);
    assert_eq!(
        saved.components.as_ref().unwrap()[1].factor.to_ne_bytes(),
        1.0f64.to_ne_bytes()
    );
    assert_eq!(saved.start_time.to_string(), "2022-03-01 12:00:00 UTC");
    assert_eq!(
        saved.stop_time.unwrap().to_string(),
        "2022-03-01 13:00:00 UTC"
    );
    assert_eq!(saved.runtime.unwrap(), 3600);
}

#[tokio::test]
async fn add_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    #[derive(serde::Serialize, serde::Deserialize, Clone)]
    pub struct TestRecord {
        pub record_id: Option<String>,
        pub site_id: Option<String>,
        pub user_id: Option<String>,
        pub group_id: Option<String>,
        pub components: Option<Vec<Component>>,
        pub start_time: Option<DateTime<Utc>>,
        pub stop_time: Option<DateTime<Utc>>,
        pub runtime: Option<i64>,
    }

    let record = TestRecord {
        record_id: Some("hpc-1337".into()),
        site_id: Some("cluster1".into()),
        user_id: Some("user1".into()),
        group_id: Some("group1".into()),
        components: Some(vec![
            Component {
                name: "CPU".into(),
                amount: 10,
                factor: 1.3,
            },
            Component {
                name: "Memory".into(),
                amount: 120,
                factor: 1.0,
            },
        ]),
        start_time: Some(
            DateTime::parse_from_rfc3339("2022-03-01T12:00:00-00:00")
                .unwrap()
                .with_timezone(&Utc),
        ),
        stop_time: Some(
            DateTime::parse_from_rfc3339("2022-03-01T13:00:00-00:00")
                .unwrap()
                .with_timezone(&Utc),
        ),
        runtime: Some(3600),
    };

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
            // Additional customized error message on test failure
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
    let body = RecordUpdate {
        record_id: "does_not_exist".into(),
        site_id: "cluster1".into(),
        user_id: "user1".into(),
        group_id: "group1".into(),
        components: vec![
            Component {
                name: "CPU".into(),
                amount: 10,
                factor: 1.3,
            },
            Component {
                name: "Memory".into(),
                amount: 120,
                factor: 1.0,
            },
        ],
        start_time: None,
        stop_time: DateTime::parse_from_rfc3339("2022-03-01T13:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc),
    };

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
    let body = RecordAdd {
        record_id: "hpc-1234".into(),
        site_id: "cluster1".into(),
        user_id: "user1".into(),
        group_id: "group1".into(),
        components: vec![
            Component {
                name: "CPU".into(),
                amount: 10,
                factor: 1.3,
            },
            Component {
                name: "Memory".into(),
                amount: 120,
                factor: 1.0,
            },
        ],
        start_time: DateTime::parse_from_rfc3339("2022-03-01T12:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc),
        stop_time: None,
    };

    let response = client
        .post(&format!("{}/add", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    // Update this record
    let body = RecordUpdate {
        record_id: "hpc-1234".into(),
        site_id: "cluster1".into(),
        user_id: "user1".into(),
        group_id: "group1".into(),
        components: vec![
            Component {
                name: "CPU".into(),
                amount: 10,
                factor: 1.3,
            },
            Component {
                name: "Memory".into(),
                amount: 120,
                factor: 1.0,
            },
        ],
        start_time: None,
        stop_time: DateTime::parse_from_rfc3339("2022-03-01T13:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc),
    };

    let response = client
        .post(&format!("{}/update", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!(
        r#"SELECT
           record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
           start_time, stop_time, runtime
           FROM accounting
           WHERE record_id = $1
        "#,
        "hpc-1234"
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.record_id, "hpc-1234");
    assert_eq!(saved.site_id.unwrap(), "cluster1");
    assert_eq!(saved.user_id.unwrap(), "user1");
    assert_eq!(saved.group_id.unwrap(), "group1");
    assert_eq!(saved.components.as_ref().unwrap()[0].name, "CPU");
    assert_eq!(saved.components.as_ref().unwrap()[0].amount, 10);
    assert_eq!(
        saved.components.as_ref().unwrap()[0].factor.to_ne_bytes(),
        1.3f64.to_ne_bytes()
    );
    assert_eq!(saved.components.as_ref().unwrap()[1].name, "Memory");
    assert_eq!(saved.components.as_ref().unwrap()[1].amount, 120);
    assert_eq!(
        saved.components.as_ref().unwrap()[1].factor.to_ne_bytes(),
        1.0f64.to_ne_bytes()
    );
    assert_eq!(saved.start_time.to_string(), "2022-03-01 12:00:00 UTC");
    assert_eq!(
        saved.stop_time.unwrap().to_string(),
        "2022-03-01 13:00:00 UTC"
    );
    assert_eq!(saved.runtime.unwrap(), 3600);
}

#[tokio::test]
async fn get_returns_a_200_and_list_of_records() {
    // Arange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // First send a couple of records
    let record = RecordAdd {
        record_id: "hpc-1337".into(),
        site_id: "cluster1".into(),
        user_id: "user1".into(),
        group_id: "group1".into(),
        components: vec![
            Component {
                name: "CPU".into(),
                amount: 10,
                factor: 1.3,
            },
            Component {
                name: "Memory".into(),
                amount: 120,
                factor: 1.0,
            },
        ],
        start_time: DateTime::parse_from_rfc3339("2022-03-01T12:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc),
        stop_time: Some(
            DateTime::parse_from_rfc3339("2022-03-01T13:00:00-00:00")
                .unwrap()
                .with_timezone(&Utc),
        ),
    };

    let test_cases = vec![
        {
            let mut r = record.clone();
            r.record_id = "r1".to_string();
            r
        },
        {
            let mut r = record.clone();
            r.record_id = "r2".to_string();
            r
        },
        {
            let mut r = record.clone();
            r.record_id = "r3".to_string();
            r
        },
    ];

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

    let received_records = response.json::<Vec<Record>>().await.unwrap();

    for (record, received) in test_cases.iter().zip(received_records.iter()) {
        assert_eq!(record.record_id, received.record_id);
        assert_eq!(record.site_id, *received.site_id.as_ref().unwrap());
        assert_eq!(record.user_id, *received.user_id.as_ref().unwrap());
        assert_eq!(record.group_id, *received.group_id.as_ref().unwrap());
        assert_eq!(record.components, *received.components.as_ref().unwrap());
        assert_eq!(record.start_time, received.start_time);
        assert_eq!(
            record.stop_time.unwrap(),
            *received.stop_time.as_ref().unwrap()
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
    let record = RecordAdd {
        record_id: "hpc-1337".into(),
        site_id: "cluster1".into(),
        user_id: "user1".into(),
        group_id: "group1".into(),
        components: vec![
            Component {
                name: "CPU".into(),
                amount: 10,
                factor: 1.3,
            },
            Component {
                name: "Memory".into(),
                amount: 120,
                factor: 1.0,
            },
        ],
        start_time: DateTime::parse_from_rfc3339("2022-03-01T12:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc),
        stop_time: Some(
            DateTime::parse_from_rfc3339("2022-03-01T13:00:00-00:00")
                .unwrap()
                .with_timezone(&Utc),
        ),
    };

    let test_cases = vec![
        {
            let mut r = record.clone();
            r.record_id = "r1".to_string();
            r.start_time = DateTime::parse_from_rfc3339("2022-03-01T12:00:00-00:00")
                .unwrap()
                .with_timezone(&Utc);
            r
        },
        {
            let mut r = record.clone();
            r.record_id = "r2".to_string();
            r.start_time = DateTime::parse_from_rfc3339("2022-03-02T12:00:00-00:00")
                .unwrap()
                .with_timezone(&Utc);
            r
        },
        {
            let mut r = record.clone();
            r.record_id = "r3".to_string();
            r.start_time = DateTime::parse_from_rfc3339("2022-03-03T12:00:00-00:00")
                .unwrap()
                .with_timezone(&Utc);
            r
        },
    ];

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
        .get(&format!(
            "{}/get/started/since/2022-03-02T00:00:00-00:00",
            &app.address
        ))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(200, response.status().as_u16());

    let received_records = response.json::<Vec<Record>>().await.unwrap();

    for (record, received) in test_cases.iter().skip(1).zip(received_records.iter()) {
        assert_eq!(record.record_id, received.record_id);
        assert_eq!(record.site_id, *received.site_id.as_ref().unwrap());
        assert_eq!(record.user_id, *received.user_id.as_ref().unwrap());
        assert_eq!(record.group_id, *received.group_id.as_ref().unwrap());
        assert_eq!(record.components, *received.components.as_ref().unwrap());
        assert_eq!(record.start_time, received.start_time);
        assert_eq!(
            record.stop_time.unwrap(),
            *received.stop_time.as_ref().unwrap()
        );
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
    let record = RecordAdd {
        record_id: "hpc-1337".into(),
        site_id: "cluster1".into(),
        user_id: "user1".into(),
        group_id: "group1".into(),
        components: vec![
            Component {
                name: "CPU".into(),
                amount: 10,
                factor: 1.3,
            },
            Component {
                name: "Memory".into(),
                amount: 120,
                factor: 1.0,
            },
        ],
        start_time: DateTime::parse_from_rfc3339("2022-03-01T12:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc),
        stop_time: Some(
            DateTime::parse_from_rfc3339("2022-03-01T13:00:00-00:00")
                .unwrap()
                .with_timezone(&Utc),
        ),
    };

    let test_cases = vec![
        {
            let mut r = record.clone();
            r.record_id = "r1".to_string();
            r.stop_time = Some(
                DateTime::parse_from_rfc3339("2022-03-01T13:00:00-00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            );
            r
        },
        {
            let mut r = record.clone();
            r.record_id = "r2".to_string();
            r.stop_time = Some(
                DateTime::parse_from_rfc3339("2022-03-02T13:00:00-00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            );
            r
        },
        {
            let mut r = record.clone();
            r.record_id = "r3".to_string();
            r.stop_time = Some(
                DateTime::parse_from_rfc3339("2022-03-03T13:00:00-00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            );
            r
        },
    ];

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
        .get(&format!(
            "{}/get/stopped/since/2022-03-02T00:00:00-00:00",
            &app.address
        ))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(200, response.status().as_u16());

    let received_records = response.json::<Vec<Record>>().await.unwrap();

    for (record, received) in test_cases.iter().skip(1).zip(received_records.iter()) {
        assert_eq!(record.record_id, received.record_id);
        assert_eq!(record.site_id, *received.site_id.as_ref().unwrap());
        assert_eq!(record.user_id, *received.user_id.as_ref().unwrap());
        assert_eq!(record.group_id, *received.group_id.as_ref().unwrap());
        assert_eq!(record.components, *received.components.as_ref().unwrap());
        assert_eq!(record.start_time, received.start_time);
        assert_eq!(
            record.stop_time.unwrap(),
            *received.stop_time.as_ref().unwrap()
        );
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
