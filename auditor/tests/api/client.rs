use crate::helpers::spawn_app;
use auditor::client::AuditorClient;
use auditor::domain::{Component, Record, RecordAdd, RecordTest, RecordUpdate};
use chrono::{TimeZone, Utc};
use fake::{Fake, Faker};

#[tokio::test]
async fn add_records() {
    // Arange
    let app = spawn_app().await;
    let client = AuditorClient::from_connection_string(&app.address).unwrap();

    let mut test_cases_comp: Vec<RecordTest> = (0..100)
        .into_iter()
        .map(|_| Faker.fake::<RecordTest>())
        .collect();
    let test_cases: Vec<RecordAdd> = test_cases_comp
        .iter()
        .cloned()
        .map(RecordAdd::try_from)
        .map(Result::unwrap)
        .collect();

    for record in test_cases {
        client.add(&record).await.unwrap();
    }

    let mut saved_records = sqlx::query_as!(
        Record,
        r#"SELECT
           record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
           start_time as "start_time?", stop_time, runtime
           FROM accounting
        "#
    )
    .fetch_all(&app.db_pool)
    .await
    .expect("Failed to fetch data.");

    // make sure they are both sorted
    test_cases_comp.sort_by(|a, b| {
        a.record_id
            .as_ref()
            .unwrap()
            .cmp(b.record_id.as_ref().unwrap())
    });
    saved_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

    for (i, (record, saved)) in test_cases_comp.iter().zip(saved_records.iter()).enumerate() {
        assert_eq!(
            record,
            saved,
            "Check {}: Record {} and {} did not match.",
            i,
            record.record_id.as_ref().unwrap(),
            saved.record_id
        );
    }
}

#[tokio::test]
async fn update_records() {
    // Arange
    let app = spawn_app().await;
    let client = AuditorClient::from_connection_string(&app.address).unwrap();

    let mut test_cases_comp: Vec<RecordTest> = (0..100)
        .into_iter()
        .map(|_| Faker.fake::<RecordTest>())
        .collect();

    let test_cases: Vec<RecordAdd> = test_cases_comp
        .iter()
        .cloned()
        .map(RecordAdd::try_from)
        .map(Result::unwrap)
        .collect();

    for mut record in test_cases {
        record.stop_time = None;
        client.add(&record).await.unwrap();
    }

    let test_cases: Vec<RecordUpdate> = test_cases_comp
        .iter()
        .cloned()
        .map(RecordUpdate::try_from)
        .map(Result::unwrap)
        .collect();

    for record in test_cases {
        client.update(&record).await.unwrap();
    }

    let mut saved_records = sqlx::query_as!(
        Record,
        r#"SELECT
           record_id, site_id, user_id, group_id, components as "components: Vec<Component>",
           start_time as "start_time?", stop_time, runtime
           FROM accounting
        "#
    )
    .fetch_all(&app.db_pool)
    .await
    .expect("Failed to fetch data.");

    // make sure they are both sorted
    test_cases_comp.sort_by(|a, b| {
        a.record_id
            .as_ref()
            .unwrap()
            .cmp(b.record_id.as_ref().unwrap())
    });
    saved_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

    for (i, (record, saved)) in test_cases_comp.iter().zip(saved_records.iter()).enumerate() {
        assert_eq!(
            record,
            saved,
            "Check {}: Record {} and {} did not match.",
            i,
            record.record_id.as_ref().unwrap(),
            saved.record_id
        );
    }
}

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

#[tokio::test]
async fn get_started_since_returns_a_list_of_records() {
    let app = spawn_app().await;
    let client = AuditorClient::from_connection_string(&app.address).unwrap();

    let mut test_cases: Vec<RecordTest> = (1..=31)
        .into_iter()
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                .with_record_id(format!("r{:0>2}", i))
                .with_start_time(format!("2022-03-{:0>2}T12:00:00-00:00", i))
        })
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

    let mut received_records = client
        .get_started_since(&Utc.ymd(2022, 3, 15).and_hms_milli(0, 0, 0, 0))
        .await
        .unwrap();

    assert_eq!(received_records.len(), 17);

    // make sure they are both sorted
    test_cases.sort_by(|a, b| {
        a.record_id
            .as_ref()
            .unwrap()
            .cmp(b.record_id.as_ref().unwrap())
    });
    received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

    for (i, (record, received)) in test_cases
        .iter()
        .skip(14)
        .zip(received_records.iter())
        .enumerate()
    {
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
async fn get_stopped_since_returns_a_list_of_records() {
    let app = spawn_app().await;
    let client = AuditorClient::from_connection_string(&app.address).unwrap();

    let mut test_cases: Vec<RecordTest> = (1..=31)
        .into_iter()
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                .with_record_id(format!("r{:0>2}", i))
                .with_stop_time(format!("2022-03-{:0>2}T12:00:00-00:00", i))
        })
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

    let mut received_records = client
        .get_stopped_since(&Utc.ymd(2022, 3, 15).and_hms_milli(0, 0, 0, 0))
        .await
        .unwrap();

    assert_eq!(received_records.len(), 17);

    // make sure they are both sorted
    test_cases.sort_by(|a, b| {
        a.record_id
            .as_ref()
            .unwrap()
            .cmp(b.record_id.as_ref().unwrap())
    });
    received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

    for (i, (record, received)) in test_cases
        .iter()
        .skip(14)
        .zip(received_records.iter())
        .enumerate()
    {
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
