use crate::helpers::spawn_app;
use auditor::domain::{Record, RecordAdd, RecordTest, RecordUpdate};
use auditor_client::{AuditorClientBuilder, Operator, QueryBuilder};
use chrono::{TimeZone, Utc};
use fake::{Fake, Faker};

#[tokio::test]
async fn add_records() {
    // Arrange
    let app = spawn_app().await;
    let client = AuditorClientBuilder::new()
        .connection_string(&app.address)
        .build()
        .unwrap();

    let mut test_cases_comp: Vec<RecordTest> =
        (0..100).map(|_| Faker.fake::<RecordTest>()).collect();
    let test_cases: Vec<RecordAdd> = test_cases_comp
        .iter()
        .cloned()
        .map(RecordAdd::try_from)
        .map(Result::unwrap)
        .collect();

    for record in test_cases {
        client.add(&record).await.unwrap();
    }

    let response = app.get_records().await;

    let mut received_records = response.json::<Vec<Record>>().await.unwrap();

    // make sure they are both sorted
    test_cases_comp.sort_by(|a, b| {
        a.record_id
            .as_ref()
            .unwrap()
            .cmp(b.record_id.as_ref().unwrap())
    });

    received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

    for (i, (record, received)) in test_cases_comp
        .iter()
        .zip(received_records.iter())
        .enumerate()
    {
        assert_eq!(
            record,
            received,
            "Check {i}: Record {} and {} did not match.",
            record.record_id.as_ref().unwrap(),
            received.record_id
        );
    }
}

#[tokio::test]
async fn update_records() {
    // Arrange
    let app = spawn_app().await;
    let client = AuditorClientBuilder::new()
        .connection_string(&app.address)
        .build()
        .unwrap();

    let mut test_cases_comp: Vec<RecordTest> =
        (0..100).map(|_| Faker.fake::<RecordTest>()).collect();

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

    let response = app.get_records().await;

    let mut received_records = response.json::<Vec<Record>>().await.unwrap();

    // make sure they are both sorted
    test_cases_comp.sort_by(|a, b| {
        a.record_id
            .as_ref()
            .unwrap()
            .cmp(b.record_id.as_ref().unwrap())
    });

    received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

    for (i, (record, received)) in test_cases_comp
        .iter()
        .zip(received_records.iter())
        .enumerate()
    {
        assert_eq!(
            record,
            received,
            "Check {i}: Record {} and {} did not match.",
            record.record_id.as_ref().unwrap(),
            received.record_id
        );
    }
}

#[tokio::test]
async fn get_returns_empty_list_of_records() {
    // Arrange
    let app = spawn_app().await;
    let client = AuditorClientBuilder::new()
        .connection_string(&app.address)
        .build()
        .unwrap();

    let records = client.get().await.unwrap();

    assert!(records.is_empty());
}

#[tokio::test]
async fn get_returns_a_list_of_records() {
    let app = spawn_app().await;
    let client = AuditorClientBuilder::new()
        .connection_string(&app.address)
        .build()
        .unwrap();

    let mut test_cases_comp: Vec<RecordTest> =
        (0..100).map(|_| Faker.fake::<RecordTest>()).collect();

    let test_cases: Vec<RecordAdd> = test_cases_comp
        .iter()
        .cloned()
        .map(RecordAdd::try_from)
        .map(Result::unwrap)
        .collect();

    for record in test_cases {
        client.add(&record).await.unwrap();
    }

    // make sure they are both sorted
    let response = app.get_records().await;

    let mut received_records = response.json::<Vec<Record>>().await.unwrap();

    assert_eq!(received_records.len(), 100);
    // make sure they are both sorted
    test_cases_comp.sort_by(|a, b| {
        a.record_id
            .as_ref()
            .unwrap()
            .cmp(b.record_id.as_ref().unwrap())
    });

    received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

    for (i, (record, received)) in test_cases_comp
        .iter()
        .zip(received_records.iter())
        .enumerate()
    {
        assert_eq!(
            record,
            received,
            "Check {i}: Record {} and {} did not match.",
            record.record_id.as_ref().unwrap(),
            received.record_id
        );
    }
}

#[tokio::test]
async fn get_started_since_returns_a_list_of_records() {
    let app = spawn_app().await;
    let client = AuditorClientBuilder::new()
        .connection_string(&app.address)
        .build()
        .unwrap();

    let mut test_cases_comp: Vec<RecordTest> = (1..=31)
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                .with_record_id(format!("r{i:0>2}"))
                .with_start_time(format!("2022-03-{i:0>2}T12:00:00-00:00"))
        })
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

    let date = Utc.with_ymd_and_hms(2022, 3, 15, 0, 0, 0).unwrap();

    let mut received_records = QueryBuilder::new()
        .with_start_time(Operator::default().gte(date.into()))
        .get(client)
        .await
        .unwrap();

    assert_eq!(received_records.len(), 17);

    // make sure they are both sorted
    test_cases_comp.sort_by(|a, b| {
        a.record_id
            .as_ref()
            .unwrap()
            .cmp(b.record_id.as_ref().unwrap())
    });
    received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

    for (i, (record, received)) in test_cases_comp
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
    let client = AuditorClientBuilder::new()
        .connection_string(&app.address)
        .build()
        .unwrap();

    let mut test_cases_comp: Vec<RecordTest> = (1..=31)
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                .with_record_id(format!("r{i:0>2}"))
                .with_stop_time(format!("2022-03-{i:0>2}T12:00:00-00:00"))
        })
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

    let date = Utc.with_ymd_and_hms(2022, 3, 15, 0, 0, 0).unwrap();
    let mut received_records = QueryBuilder::new()
        .with_stop_time(Operator::default().gte(date.into()))
        .get(client)
        .await
        .unwrap();

    assert_eq!(received_records.len(), 17);

    // make sure they are both sorted
    test_cases_comp.sort_by(|a, b| {
        a.record_id
            .as_ref()
            .unwrap()
            .cmp(b.record_id.as_ref().unwrap())
    });
    received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

    for (i, (record, received)) in test_cases_comp
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
