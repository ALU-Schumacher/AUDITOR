use crate::helpers::spawn_app;
use auditor::domain::{Record, RecordTest};
use fake::{Fake, Faker};

#[tokio::test]
async fn get_returns_a_200_and_list_of_records() {
    // Arange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // First send a couple of records
    let mut test_cases: Vec<RecordTest> = (0..100)
        .into_iter()
        .map(|_| Faker.fake::<RecordTest>())
        .collect();

    for case in test_cases.iter() {
        let response = client
            .post(&format!("{}/add", &app.address))
            .header("Content-Type", "application/json")
            .json(&case)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(200, response.status().as_u16());
    }

    let response = client
        .get(&format!("{}/get", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");
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
async fn get_returns_a_200_and_no_records() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/get", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(200, response.status().as_u16());

    let received_records = response.json::<Vec<Record>>().await.unwrap();

    assert!(received_records.is_empty());
}
