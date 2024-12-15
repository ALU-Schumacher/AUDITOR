use crate::helpers::spawn_app;
use auditor::domain::{RecordDatabase, RecordTest};
use fake::{Fake, Faker};

#[tokio::test]
async fn add_returns_a_200_for_valid_json_data() {
    // Arrange
    let app = spawn_app().await;

    // Act
    for _ in 0..100 {
        let body: RecordTest = Faker.fake();

        let response = app.add_record(&body).await;

        assert_eq!(200, response.status().as_u16());

        let saved = sqlx::query_as!(
            RecordDatabase,
            r#"SELECT record_id,
                  meta,
                  components,
                  start_time,
                  stop_time,
                  runtime
           FROM auditor_accounting
           WHERE record_id = $1
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

        let saved: Vec<_> = sqlx::query!(r#"SELECT record_id FROM auditor_accounting"#,)
            .fetch_all(&app.db_pool)
            .await
            .expect("Failed to fetch data");

        assert_eq!(saved.len(), 0);
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

#[tokio::test]
async fn bulk_insert_records() {
    let app = spawn_app().await;

    let records: Vec<RecordTest> = (0..100).map(|_| Faker.fake()).collect();

    let response = app.bulk_insert(&records).await;

    assert_eq!(200, response.status().as_u16());

    for record in records {
        let saved = sqlx::query_as!(
            RecordDatabase,
            r#"SELECT record_id,
                  meta,
                  components,
                  start_time,
                  stop_time,
                  runtime
           FROM auditor_accounting
           WHERE record_id = $1
            "#,
            record.record_id.as_ref().unwrap(),
        )
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch data")
        .try_into()
        .expect("Failed to convert from RecordDatabase to Record");

        assert_eq!(record, saved);
    }
}

#[tokio::test]
async fn bulk_insert_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;

    let test_cases = vec![
        ("record_id is missing", {
            let records: Vec<RecordTest> = (0..2)
                .map(|_| {
                    let mut record: RecordTest = Faker.fake();
                    record.record_id = None;
                    record
                })
                .collect();
            records
        }),
        ("components is missing", {
            let records: Vec<RecordTest> = (0..2)
                .map(|_| {
                    let mut record: RecordTest = Faker.fake();
                    record.components = None;
                    record
                })
                .collect();
            records
        }),
        ("start_time is missing", {
            let records: Vec<RecordTest> = (0..2)
                .map(|_| {
                    let mut record: RecordTest = Faker.fake();
                    record.start_time = None;
                    record
                })
                .collect();
            records
        }),
    ];

    for (error_message, invalid_body) in test_cases {
        let response = app.add_record(&invalid_body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {error_message}."
        );

        let saved: Vec<_> = sqlx::query!(r#"SELECT record_id FROM auditor_accounting"#,)
            .fetch_all(&app.db_pool)
            .await
            .expect("Failed to fetch data");

        assert_eq!(saved.len(), 0);
    }
}

#[tokio::test]
async fn bulk_insert_returns_a_500_for_duplicate_records() {
    let app = spawn_app().await;

    let records: Vec<RecordTest> = (0..2).map(|_| Faker.fake()).collect();

    let response = app.bulk_insert(&records).await;
    assert_eq!(200, response.status().as_u16());

    let response = app.bulk_insert(&records).await;
    assert_eq!(500, response.status().as_u16());
}
