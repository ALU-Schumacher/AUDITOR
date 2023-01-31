use crate::helpers::spawn_app;
use auditor::client::AuditorClient;
use auditor::domain::{Component, Record, RecordAdd, RecordDatabase, RecordTest, RecordUpdate};
use chrono::{TimeZone, Utc};
use fake::{Fake, Faker};

#[tokio::test]
async fn add_records() {
    // Arange
    let app = spawn_app().await;
    let client = AuditorClient::from_connection_string(&app.address).unwrap();

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

    let mut saved_records = sqlx::query_as!(
        RecordDatabase,
        r#"SELECT a.record_id,
              m.meta as "meta: Vec<(String, Vec<String>)>",
              a.components as "components: Vec<Component>",
              a.start_time as "start_time?",
              a.stop_time,
              a.runtime
       FROM accounting a
       LEFT JOIN (
           WITH subquery AS (
               SELECT m.record_id as record_id, m.key as key, array_agg(m.value) as values
               FROM meta as m
               GROUP BY m.record_id, m.key
           )
           SELECT s.record_id as record_id, array_agg(row(s.key, s.values)) as meta
           FROM subquery as s
           GROUP BY s.record_id
           ) m ON m.record_id = a.record_id
        ORDER BY a.stop_time
       "#,
    )
    .fetch_all(&app.db_pool)
    .await
    .expect("Failed to fetch data")
    .into_iter()
    .map(Record::try_from)
    .collect::<Result<Vec<Record>, _>>()
    .expect("Failed to convert from RecordDatabase to Record");

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
            "Check {i}: Record {} and {} did not match.",
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

    let mut saved_records = sqlx::query_as!(
        RecordDatabase,
        r#"SELECT a.record_id,
              m.meta as "meta: Vec<(String, Vec<String>)>",
              a.components as "components: Vec<Component>",
              a.start_time as "start_time?",
              a.stop_time,
              a.runtime
       FROM accounting a
       LEFT JOIN (
           WITH subquery AS (
               SELECT m.record_id as record_id, m.key as key, array_agg(m.value) as values
               FROM meta as m
               GROUP BY m.record_id, m.key
           )
           SELECT s.record_id as record_id, array_agg(row(s.key, s.values)) as meta
           FROM subquery as s
           GROUP BY s.record_id
           ) m ON m.record_id = a.record_id
        ORDER BY a.stop_time
       "#,
    )
    .fetch_all(&app.db_pool)
    .await
    .expect("Failed to fetch data")
    .into_iter()
    .map(Record::try_from)
    .collect::<Result<Vec<Record>, _>>()
    .expect("Failed to convert from RecordDatabase to Record");

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

    let mut test_cases: Vec<RecordTest> = (0..100).map(|_| Faker.fake::<RecordTest>()).collect();

    for record in test_cases.iter() {
        let runtime = (record.stop_time.unwrap() - record.start_time.unwrap()).num_seconds();
        let mut transaction = app.db_pool.begin().await.unwrap();

        sqlx::query_unchecked!(
            r#"
            INSERT INTO accounting (
                record_id, components, start_time, stop_time, runtime, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            record.record_id.as_ref(),
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
        .execute(&mut transaction)
        .await
        .unwrap();

        if let Some(data) = record.meta.as_ref() {
            let (rid, names, values): (Vec<String>, Vec<String>, Vec<String>) =
                itertools::multiunzip(data.iter().flat_map(|(k, v)| {
                    v.iter()
                        .map(|v| {
                            (
                                record.record_id.as_ref().unwrap().clone(),
                                k.clone(),
                                v.clone(),
                            )
                        })
                        .collect::<Vec<_>>()
                }));

            sqlx::query!(
                r#"
                INSERT INTO meta (record_id, key, value)
                SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[])
                "#,
                &rid[..],
                &names[..],
                &values[..],
            )
            .execute(&mut transaction)
            .await
            .unwrap();
        }

        transaction.commit().await.unwrap();
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
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                .with_record_id(format!("r{i:0>2}"))
                .with_start_time(format!("2022-03-{i:0>2}T12:00:00-00:00"))
        })
        .collect();

    for record in test_cases.iter() {
        let runtime = (record.stop_time.unwrap() - record.start_time.unwrap()).num_seconds();

        let mut transaction = app.db_pool.begin().await.unwrap();

        sqlx::query_unchecked!(
            r#"
            INSERT INTO accounting (
                record_id, components, start_time, stop_time, runtime, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            record.record_id.as_ref(),
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
        .execute(&mut transaction)
        .await
        .unwrap();

        if let Some(data) = record.meta.as_ref() {
            let (rid, names, values): (Vec<String>, Vec<String>, Vec<String>) =
                itertools::multiunzip(data.iter().flat_map(|(k, v)| {
                    v.iter()
                        .map(|v| {
                            (
                                record.record_id.as_ref().unwrap().clone(),
                                k.clone(),
                                v.clone(),
                            )
                        })
                        .collect::<Vec<_>>()
                }));

            sqlx::query!(
                r#"
                INSERT INTO meta (record_id, key, value)
                SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[])
                "#,
                &rid[..],
                &names[..],
                &values[..],
            )
            .execute(&mut transaction)
            .await
            .unwrap();
        }

        transaction.commit().await.unwrap();
    }

    let mut received_records = client
        .get_started_since(&Utc.with_ymd_and_hms(2022, 3, 15, 0, 0, 0).unwrap())
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
        .map(|i| {
            Faker
                .fake::<RecordTest>()
                .with_record_id(format!("r{i:0>2}"))
                .with_stop_time(format!("2022-03-{i:0>2}T12:00:00-00:00"))
        })
        .collect();

    for record in test_cases.iter() {
        let runtime = (record.stop_time.unwrap() - record.start_time.unwrap()).num_seconds();

        let mut transaction = app.db_pool.begin().await.unwrap();

        sqlx::query_unchecked!(
            r#"
            INSERT INTO accounting (
                record_id, components, start_time, stop_time, runtime, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            record.record_id.as_ref(),
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
        .execute(&mut transaction)
        .await
        .unwrap();

        if let Some(data) = record.meta.as_ref() {
            let (rid, names, values): (Vec<String>, Vec<String>, Vec<String>) =
                itertools::multiunzip(data.iter().flat_map(|(k, v)| {
                    v.iter()
                        .map(|v| {
                            (
                                record.record_id.as_ref().unwrap().clone(),
                                k.clone(),
                                v.clone(),
                            )
                        })
                        .collect::<Vec<_>>()
                }));

            sqlx::query!(
                r#"
                INSERT INTO meta (record_id, key, value)
                SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[])
                "#,
                &rid[..],
                &names[..],
                &values[..],
            )
            .execute(&mut transaction)
            .await
            .unwrap();
        }

        transaction.commit().await.unwrap();
    }

    let mut received_records = client
        .get_stopped_since(&Utc.with_ymd_and_hms(2022, 3, 15, 0, 0, 0).unwrap())
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
