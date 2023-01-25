use crate::helpers::spawn_app;
use auditor::domain::{Component, RecordDatabase, RecordTest};
use fake::{Fake, Faker};

#[tokio::test]
async fn add_returns_a_200_for_valid_json_data() {
    // Arange
    let app = spawn_app().await;

    // Act
    for _ in 0..100 {
        let body: RecordTest = Faker.fake();

        let response = app.add_record(&body).await;

        assert_eq!(200, response.status().as_u16());

        let saved = sqlx::query_as!(
            RecordDatabase,
            r#"SELECT a.record_id,
                      m.meta as "meta: Vec<(String, Vec<String>)>",
                      a.site_id,
                      a.user_id,
                      a.group_id,
                      a.components as "components: Vec<Component>",
                      a.start_time as "start_time?",
                      a.stop_time,
                      a.runtime
               FROM accounting a
               LEFT JOIN (
                   SELECT m.record_id as record_id, array_agg(row(m.key, m.value)) as meta 
                   FROM meta as m
                   GROUP BY m.record_id
                   ) m ON m.record_id = a.record_id
               WHERE a.record_id = $1
            "#,
            body.record_id.as_ref().unwrap(),
        )
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch data")
        .try_into()
        .expect("Failed to convert from RecordDatabase to Record");

        assert_eq!(body, saved);
    }
}

#[tokio::test]
async fn add_returns_a_400_for_invalid_json_data() {
    // Arange
    let app = spawn_app().await;

    let forbidden_strings: Vec<String> = ['/', '(', ')', '"', '<', '>', '\\', '{', '}']
        .into_iter()
        .map(|s| format!("test{s}test"))
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

            let response = app.add_record(&body).await;

            assert_eq!(400, response.status().as_u16());

            let saved: Vec<_> = sqlx::query_as!(
                RecordDatabase,
                r#"SELECT a.record_id,
                      m.meta as "meta: Vec<(String, Vec<String>)>",
                      a.site_id,
                      a.user_id,
                      a.group_id,
                      a.components as "components: Vec<Component>",
                      a.start_time as "start_time?",
                      a.stop_time,
                      a.runtime
               FROM accounting a
               LEFT JOIN (
                   SELECT m.record_id as record_id, array_agg(row(m.key, m.value)) as meta 
                   FROM meta as m
                   GROUP BY m.record_id
                   ) m ON m.record_id = a.record_id
               WHERE a.record_id = $1
            "#,
                body.record_id.as_ref().unwrap(),
            )
            .fetch_all(&app.db_pool)
            .await
            .expect("Failed to fetch data");

            assert_eq!(saved.len(), 0);
        }
    }
}

#[tokio::test]
async fn add_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;

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
        let response = app.add_record(&invalid_body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {error_message}."
        );
    }
}

#[tokio::test]
async fn add_returns_a_500_for_duplicate_records() {
    // Arrange
    let app = spawn_app().await;

    let record: RecordTest = Faker.fake();

    let response = app.add_record(&record).await;
    assert_eq!(200, response.status().as_u16());

    let response = app.add_record(&record).await;
    assert_eq!(500, response.status().as_u16());
}
