use crate::helpers::spawn_app;
use auditor::domain::{Component, Record, RecordTest};
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
            Record,
            r#"SELECT
           record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
           start_time as "start_time?", stop_time, runtime
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

            let response = app.add_record(&body).await;

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
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}
