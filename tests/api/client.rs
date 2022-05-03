use crate::helpers::spawn_app;
use auditor::client::AuditorClient;
use auditor::domain::{Component, RecordTest};
use chrono::Utc;
use fake::{Fake, Faker};

#[tokio::test]
async fn get_returns_empty_list_of_records() {
    // Arange
    let app = spawn_app().await;
    let client = AuditorClient::from_connection_string(&app.address).unwrap();

    let records = client.get().await.unwrap();

    assert!(records.is_empty());
}

#[tokio::test]
async fn get_returns_a_list_of_records() {
    let app = spawn_app().await;
    let client = AuditorClient::from_connection_string(&app.address).unwrap();

    let mut test_cases: Vec<RecordTest> = (0..100)
        .into_iter()
        .map(|_| Faker.fake::<RecordTest>())
        .collect();

    for record in test_cases.iter() {
        let runtime = (record.stop_time.unwrap() - record.start_time.unwrap()).num_seconds();
        sqlx::query_unchecked!(
            r#"
            INSERT INTO accounting (
                record_id, site_id, user_id, group_id,
                components, start_time, stop_time, runtime, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            record.record_id.as_ref(),
            record.site_id.as_ref(),
            record.user_id.as_ref(),
            record.group_id.as_ref(),
            record
                .components
                .as_ref()
                .unwrap()
                .iter()
                .cloned()
                .map(Component::try_from)
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            record.start_time,
            record.stop_time,
            runtime,
            Utc::now()
        )
        .execute(&app.db_pool)
        .await
        .unwrap();
    }

    let mut received_records = client.get().await.unwrap();

    assert_eq!(received_records.len(), 100);

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
