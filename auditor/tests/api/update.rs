use crate::helpers::spawn_app;
use auditor::domain::{Record, RecordDatabase, RecordTest};
use fake::{Fake, Faker};

#[tokio::test]
async fn update_returns_a_404_for_non_existing_record() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let body: RecordTest = Faker.fake();

    let response = client
        .put(format!("{}/record", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(404, response.status().as_u16());
}

#[tokio::test]
async fn update_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    // first add a record
    let mut body: RecordTest = Faker.fake();
    body = body.with_start_time("2022-03-01T12:00:00-00:00");
    body.stop_time = None;

    let response = client
        .post(format!("{}/record", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    // Update this record
    let body = body.with_stop_time("2022-03-01T13:00:00-00:00");

    let response = client
        .put(format!("{}/record", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved: Record = sqlx::query_as!(
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
        body.record_id.as_ref().unwrap()
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch data.")
    .try_into()
    .expect("Failed to convert from RecordDatabase to Record.");

    assert_eq!(saved, body);
}
