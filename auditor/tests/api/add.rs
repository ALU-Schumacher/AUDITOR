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

    for _field in ["record_id"] {
        for fs in forbidden_strings.iter() {
            // Act
            let mut body: RecordTest = Faker.fake();
            // match field {
            //     "record_id" => body.record_id = Some(fs.clone()),
            //     _ => (),
            // }
            body.record_id = Some(fs.clone());

            let response = app.add_record(&body).await;

            assert_eq!(400, response.status().as_u16());

            let saved: Vec<_> = sqlx::query!(r#"SELECT record_id FROM accounting"#,)
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

        let saved: Vec<_> = sqlx::query!(r#"SELECT record_id FROM accounting"#,)
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
