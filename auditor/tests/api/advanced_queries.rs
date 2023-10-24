use crate::helpers::spawn_app;
use auditor::client::{QueryBuilder, TimeOperator};
use auditor::domain::{Record, RecordTest};
use chrono::{TimeZone, Utc};
use fake::{Fake, Faker};

#[tokio::test]
async fn get_advanced_queries_returns_a_200_and_list_of_records() {
    // Arange
    let app = spawn_app().await;

    // First send a couple of records
    let test_cases = (1..10)
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                // Giving a name which is sorted the same as the time is useful for asserting later
                .with_record_id(format!("r{i}"))
                .with_start_time(format!("2022-10-0{i}T12:00:00-00:00"))
        })
        .collect::<Vec<_>>();

    for case in test_cases.iter() {
        let response = app.add_record(&case).await;

        assert_eq!(200, response.status().as_u16());
    }

    // Try different start dates and receive records
    for i in 1..10 {
        let query = QueryBuilder::new()
            .with_start_time(
                TimeOperator::default()
                    .gte(Utc.with_ymd_and_hms(2022, 10, i, 9, 47, 0).unwrap().into()),
            )
            .build();

        let response = app.advanced_queries(query).await;
        println!("{:?}", response);

        assert_eq!(200, response.status().as_u16());

        let mut received_records = response.json::<Vec<Record>>().await.unwrap();

        // make sure they are both sorted
        received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

        for (j, (record, received)) in test_cases
            .iter()
            .skip(usize::try_from(i).unwrap() - 1)
            .zip(received_records.iter())
            .enumerate()
        {
            assert_eq!(
                record,
                received,
                "Check {i}|{j}: Record {} and {} did not match.",
                record.record_id.as_ref().unwrap(),
                received.record_id
            );
        }
    }
}

#[tokio::test]
async fn get_started_since_returns_a_list_of_sorted_records() {
    // Arange
    let app = spawn_app().await;

    // First send a couple of records
    let test_cases = (1..10)
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                // Giving a name which is sorted the same as the time is useful for asserting later
                .with_record_id(format!("r{i}"))
                .with_start_time(format!("2022-10-0{i}T12:00:00-00:00"))
        })
        .collect::<Vec<_>>();

    for case in test_cases.iter() {
        let response = app.add_record(&case).await;

        assert_eq!(200, response.status().as_u16());
    }

    // Try different start dates and receive records
    for i in 1..10 {
        let query = QueryBuilder::new()
            .with_start_time(
                TimeOperator::default()
                    .gte(Utc.with_ymd_and_hms(2022, 10, i, 9, 47, 0).unwrap().into()),
            )
            .build();

        let response = app.advanced_queries(query).await;

        assert_eq!(200, response.status().as_u16());

        let received_records = response.json::<Vec<Record>>().await.unwrap();

        // make sure the test cases are sorted by stop_time
        let mut tmp_test_cases = test_cases
            .iter()
            .skip(usize::try_from(i).unwrap() - 1)
            .cloned()
            .collect::<Vec<_>>();
        tmp_test_cases.sort_by(|a, b| a.stop_time.cmp(&b.stop_time));

        for (j, (record, received)) in tmp_test_cases
            .iter()
            .zip(received_records.iter())
            .enumerate()
        {
            assert_eq!(
                record,
                received,
                "Check {}|{}: Record {} and {} did not match.",
                i,
                j,
                record.record_id.as_ref().unwrap(),
                received.record_id
            );
        }
    }
}

#[tokio::test]
async fn get_started_since_returns_a_200_and_no_records() {
    let app = spawn_app().await;

    let query = QueryBuilder::new()
        .with_start_time(
            TimeOperator::default()
                .gte(Utc.with_ymd_and_hms(2022, 10, 3, 9, 47, 0).unwrap().into()),
        )
        .build();

    let response = app.advanced_queries(query).await;

    assert_eq!(200, response.status().as_u16());

    let received_records = response.json::<Vec<Record>>().await.unwrap();

    assert!(received_records.is_empty());
}

#[tokio::test]
async fn get_stopped_since_returns_a_200_and_list_of_records() {
    // Arange
    let app = spawn_app().await;

    // First send a couple of records
    let test_cases = (1..10)
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                // Giving a name which is sorted the same as the time is useful for asserting later
                .with_record_id(format!("r{i}"))
                .with_stop_time(format!("2022-10-0{i}T12:00:00-00:00"))
        })
        .collect::<Vec<_>>();

    for case in test_cases.iter() {
        let response = app.add_record(&case).await;

        assert_eq!(200, response.status().as_u16());
    }

    // Try different start dates and receive records
    for i in 1..10 {
        let query = QueryBuilder::new()
            .with_stop_time(
                TimeOperator::default()
                    .gte(Utc.with_ymd_and_hms(2022, 10, i, 9, 47, 0).unwrap().into()),
            )
            .build();

        let response = app.advanced_queries(query).await;

        assert_eq!(200, response.status().as_u16());

        let mut received_records = response.json::<Vec<Record>>().await.unwrap();

        // make sure they are both sorted
        received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

        for (j, (record, received)) in test_cases
            .iter()
            .skip(usize::try_from(i).unwrap() - 1)
            .zip(received_records.iter())
            .enumerate()
        {
            assert_eq!(
                record,
                received,
                "Check {}|{}: Record {} and {} did not match.",
                i,
                j,
                record.record_id.as_ref().unwrap(),
                received.record_id
            );
        }
    }
}

#[tokio::test]
async fn get_stopped_since_returns_a_list_of_sorted_records() {
    // Arange
    let app = spawn_app().await;

    // First send a couple of records
    let test_cases = (1..10)
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                // Giving a name which is sorted the same as the time is useful for asserting later
                .with_record_id(format!("r{i}"))
                .with_stop_time(format!("2022-10-0{i}T12:00:00-00:00"))
        })
        .collect::<Vec<_>>();

    for case in test_cases.iter() {
        let response = app.add_record(&case).await;

        assert_eq!(200, response.status().as_u16());
    }

    // Try different start dates and receive records
    for i in 1..10 {
        let query = QueryBuilder::new()
            .with_stop_time(
                TimeOperator::default()
                    .gte(Utc.with_ymd_and_hms(2022, 10, i, 9, 47, 0).unwrap().into()),
            )
            .build();

        let response = app.advanced_queries(query).await;

        assert_eq!(200, response.status().as_u16());

        let received_records = response.json::<Vec<Record>>().await.unwrap();

        // make sure the test cases are sorted by stop_time
        let mut tmp_test_cases = test_cases
            .iter()
            .skip(usize::try_from(i).unwrap() - 1)
            .cloned()
            .collect::<Vec<_>>();
        tmp_test_cases.sort_by(|a, b| a.stop_time.cmp(&b.stop_time));

        for (j, (record, received)) in tmp_test_cases
            .iter()
            .zip(received_records.iter())
            .enumerate()
        {
            assert_eq!(
                record,
                received,
                "Check {}|{}: Record {} and {} did not match.",
                i,
                j,
                record.record_id.as_ref().unwrap(),
                received.record_id
            );
        }
    }
}

#[tokio::test]
async fn get_stopped_since_returns_a_200_and_no_records() {
    let app = spawn_app().await;

    let query = QueryBuilder::new()
        .with_start_time(
            TimeOperator::default()
                .gte(Utc.with_ymd_and_hms(2022, 10, 3, 9, 47, 0).unwrap().into()),
        )
        .build();

    let response = app.advanced_queries(query).await;

    assert_eq!(200, response.status().as_u16());

    let received_records = response.json::<Vec<Record>>().await.unwrap();

    assert!(received_records.is_empty());
}
