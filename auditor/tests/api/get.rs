use crate::helpers::spawn_app;
use auditor::domain::{Record, RecordTest};
use fake::{Fake, Faker};

#[tokio::test]
async fn get_returns_a_200_and_list_of_records() {
    // Arrange
    let app = spawn_app().await;

    // First send a couple of records
    let mut test_cases: Vec<RecordTest> = (0..100).map(|_| Faker.fake::<RecordTest>()).collect();

    for case in test_cases.iter() {
        let response = app.add_record(&case).await;

        assert_eq!(200, response.status().as_u16());
    }

    let response = app.get_records().await;

    assert_eq!(200, response.status().as_u16());

    let mut received_records = response.json::<Vec<Record>>().await.unwrap();

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
async fn get_returns_a_list_of_sorted_records() {
    // Arrange
    let app = spawn_app().await;

    // First send a couple of records
    let mut test_cases: Vec<RecordTest> = (0..100).map(|_| Faker.fake::<RecordTest>()).collect();

    for case in test_cases.iter() {
        let response = app.add_record(&case).await;

        assert_eq!(200, response.status().as_u16());
    }

    let response = app.get_records().await;

    assert_eq!(200, response.status().as_u16());

    let received_records = response.json::<Vec<Record>>().await.unwrap();

    // make sure the test records are sorted
    test_cases.sort_by(|a, b| {
        a.stop_time
            .as_ref()
            .unwrap()
            .cmp(b.stop_time.as_ref().unwrap())
    });

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
async fn get_returns_a_200_and_no_records() {
    let app = spawn_app().await;

    let response = app.get_records().await;

    assert_eq!(200, response.status().as_u16());

    let received_records = response.json::<Vec<Record>>().await.unwrap();

    assert!(received_records.is_empty());
}
