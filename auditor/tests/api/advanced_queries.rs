use crate::helpers::spawn_app;
use auditor::domain::{Record, RecordTest};
use chrono::{TimeZone, Utc};
use fake::{Fake, Faker};
use std::collections::HashMap;
use urlencoding::encode;

#[tokio::test]
async fn get_advanced_queries_returns_a_200_and_list_of_records() {
    // Arrange
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
        let datetime_utc = Utc.with_ymd_and_hms(2022, 10, i, 9, 47, 0).unwrap();
        let datetime_str = datetime_utc.to_rfc3339();
        let encoded_datetime = encode(&datetime_str);
        let query = format!("start_time[gte]={encoded_datetime}");

        let (mut received_records, status) = app.advanced_queries(query).await.unwrap();

        assert_eq!(200, status);

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
    // Arrange
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
        let datetime_utc = Utc.with_ymd_and_hms(2022, 10, i, 9, 47, 0).unwrap();
        let datetime_str = datetime_utc.to_rfc3339();
        let encoded_datetime = encode(&datetime_str);
        let query = format!("start_time[gte]={encoded_datetime}");

        let (received_records, status) = app.advanced_queries(query).await.unwrap();

        assert_eq!(200, status);

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
    let datetime_utc = Utc.with_ymd_and_hms(2022, 10, 3, 9, 47, 0).unwrap();
    let datetime_str = datetime_utc.to_rfc3339();
    let encoded_datetime = encode(&datetime_str);
    let query = format!("start_time[gte]={encoded_datetime}");

    let (received_records, status) = app.advanced_queries(query).await.unwrap();

    assert_eq!(200, status);

    assert!(received_records.is_empty());
}

#[tokio::test]
async fn get_stopped_since_returns_a_200_and_list_of_records() {
    // Arrange
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
        let datetime_utc = Utc.with_ymd_and_hms(2022, 10, i, 9, 47, 0).unwrap();
        let datetime_str = datetime_utc.to_rfc3339();
        let encoded_datetime = encode(&datetime_str);
        let query = format!("stop_time[gte]={encoded_datetime}");

        let (mut received_records, status) = app.advanced_queries(query).await.unwrap();

        assert_eq!(200, status);

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
    // Arrange
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
        let datetime_utc = Utc.with_ymd_and_hms(2022, 10, i, 9, 47, 0).unwrap();
        let datetime_str = datetime_utc.to_rfc3339();
        let encoded_datetime = encode(&datetime_str);
        let query = format!("stop_time[gte]={encoded_datetime}");

        let (received_records, status) = app.advanced_queries(query).await.unwrap();

        assert_eq!(200, status);

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
    let datetime_utc = Utc.with_ymd_and_hms(2022, 10, 3, 9, 47, 0).unwrap();
    let datetime_str = datetime_utc.to_rfc3339();
    let encoded_datetime = encode(&datetime_str);
    let query = format!("start_time[gte]={encoded_datetime}");

    let (received_records, status) = app.advanced_queries(query).await.unwrap();

    assert_eq!(200, status);

    assert!(received_records.is_empty());
}

// Test should return the same meta data which are added to auditor using 'contains' operator
#[tokio::test]
async fn get_meta_queries_c_returns_a_200_and_list_of_records() {
    // Arrange
    let app = spawn_app().await;

    // First send a couple of records
    let mut meta: HashMap<String, Vec<String>> = HashMap::new();
    meta.insert("group_id".to_string(), vec!["group_1".to_string()]);
    let test_cases = (1..10)
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                // Giving a name which is sorted the same as the time is useful for asserting later
                .with_record_id(format!("r{i}"))
                .with_meta(meta.clone())
                .with_start_time(format!("2022-10-0{i}T12:00:00-00:00"))
        })
        .collect::<Vec<_>>();

    for case in test_cases.iter() {
        let response = app.add_record(&case).await;

        assert_eq!(200, response.status().as_u16());
    }

    // Try different start dates and receive records
    for i in 1..10 {
        let datetime_utc = Utc.with_ymd_and_hms(2022, 10, i, 9, 47, 0).unwrap();
        let datetime_str = datetime_utc.to_rfc3339();
        let encoded_datetime = encode(&datetime_str);
        let query = format!("start_time[gte]={encoded_datetime}&meta[group_id][c][0]=group_1");

        let (mut received_records, status) = app.advanced_queries(query).await.unwrap();

        assert_eq!(200, status);

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
async fn get_component_query_returns_a_200_and_list_of_records() {
    // Arrange
    let app = spawn_app().await;

    // First send a couple of records
    //let component_cpu = Component::new("cpu", 4)?
    //                    .with_score(Score::new("HEPSPEC06", 9.2)?);)
    let test_cases = (1..10)
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                // Giving a name which is sorted the same as the time is useful for asserting later
                .with_record_id(format!("r{i}"))
                .with_start_time(format!("2022-10-0{i}T12:00:00-00:00"))
                .with_component("cpu", 4, vec![])
        })
        .collect::<Vec<_>>();

    for case in test_cases.iter() {
        let response = app.add_record(&case).await;

        assert_eq!(200, response.status().as_u16());
    }

    // Try different start dates and receive records
    for i in 1..10 {
        let query = "component[cpu][equals]=4".to_string();

        let (mut received_records, status) = app.advanced_queries(query).await.unwrap();

        assert_eq!(200, status);

        // make sure they are both sorted
        received_records.sort_by(|a, b| a.record_id.cmp(&b.record_id));

        for (j, (record, received)) in test_cases.iter().zip(received_records.iter()).enumerate() {
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
async fn sort_by_returns_a_200_and_list_of_records() {
    // Arrange
    let app = spawn_app().await;

    // First send a couple of records
    //let component_cpu = Component::new("cpu", 4)?
    //                    .with_score(Score::new("HEPSPEC06", 9.2)?);)
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
        let query = "sort_by[desc]=start_time".to_string();

        let (received_records, status) = app.advanced_queries(query).await.unwrap();

        assert_eq!(200, status);

        for (j, (record, received)) in test_cases
            .iter()
            .rev()
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
async fn limit_query_records_returns_a_200_and_list_of_records() {
    // Arrange
    let app = spawn_app().await;

    // First send a couple of records
    //let component_cpu = Component::new("cpu", 4)?
    //                    .with_score(Score::new("HEPSPEC06", 9.2)?);)
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
        let query = "sort_by[desc]=start_time&limit=4".to_string();

        let (received_records, status) = app.advanced_queries(query).await.unwrap();

        assert_eq!(200, status);

        assert_eq!(received_records.len(), 4);

        for (j, (record, received)) in test_cases
            .iter()
            .rev()
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
async fn exact_record_id_returns_a_200_and_list_of_records() {
    // Arrange
    let app = spawn_app().await;

    // First send a couple of records
    //let component_cpu = Component::new("cpu", 4)?
    //                    .with_score(Score::new("HEPSPEC06", 9.2)?);)
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
    //let query = QueryBuilder::new().with_record_id("r3".to_string()).build();

    let query = "r3".to_string();

    let response = app.get_single_record(query).await;
    println!("{response:?}");

    assert_eq!(200, response.status().as_u16());

    let received_record = response.json::<Record>().await.unwrap();

    assert_eq!(received_record.record_id, "r3".to_string());
}
