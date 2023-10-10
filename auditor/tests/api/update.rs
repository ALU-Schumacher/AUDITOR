use crate::helpers::spawn_app;
use auditor::domain::{Component, Record, RecordDatabase, RecordTest};
use fake::{Fake, Faker};

#[tokio::test]
async fn update_returns_a_404_for_non_existing_record() {
    // Arange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let body: RecordTest = Faker.fake();

    let response = client
        .put(&format!("{}/record", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(404, response.status().as_u16());
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
        .post(&format!("{}/record", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    // Update this record
    let body = body.with_stop_time("2022-03-01T13:00:00-00:00");

    let response = client
        .put(&format!("{}/record", &app.address))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved: Record = sqlx::query_as!(
        RecordDatabase,
        r#"SELECT a.record_id,
                  m.meta as "meta: Vec<(String, Vec<String>)>",
                  css.components as "components: Vec<Component>",
                  a.start_time as "start_time?",
                  a.stop_time,
                  a.runtime
           FROM accounting a
           LEFT JOIN (
               WITH subquery AS (
                   SELECT m.record_id as record_id, m.key as key, array_agg(m.value) as values
                   FROM meta as m
                   WHERE m.record_id = $1
                   GROUP BY m.record_id, m.key
               )
               SELECT s.record_id as record_id, array_agg(row(s.key, s.values)) as meta
               FROM subquery as s
               GROUP BY s.record_id
               ) m ON m.record_id = a.record_id
           LEFT JOIN (
               WITH subquery AS (
                  SELECT 
                      c.id as cid,
                      COALESCE(array_agg(row(s.name, s.value)::score) FILTER (WHERE s.name IS NOT NULL AND s.value IS NOT NULL), '{}'::score[]) as scores
                  FROM components as c
                  LEFT JOIN components_scores as cs
                  ON c.id = cs.component_id
                  LEFT JOIN scores as s
                  ON cs.score_id = s.id
                  GROUP BY c.id
               )
               SELECT rc.record_id as id, array_agg(row(c.name, c.amount, sq.scores)::component) as components
               FROM records_components AS rc
               LEFT JOIN components as c
               ON rc.component_id = c.id
               LEFT JOIN subquery AS sq
               ON sq.cid = rc.component_id
               GROUP BY rc.record_id
           ) css ON css.id = a.id
           WHERE a.record_id = $1
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
