use crate::helpers::spawn_app;
use auditor::domain::{Record, RecordTest};
use fake::{Fake, Faker};

#[tokio::test]
async fn get_one_record_returns_a_200_and_get_one_record() {
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
        let response = app.get_single_record(format!("r{i}")).await;

        assert_eq!(200, response.status().as_u16());

        let received_record = response.json::<Record>().await.unwrap();

        assert_eq!(
            test_cases[i - 1],
            received_record,
            "Check {i}: Record {} and {} did not match",
            test_cases[i - 1].record_id.as_ref().unwrap(),
            received_record.record_id
        )
    }
}
