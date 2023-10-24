use crate::helpers::spawn_app;
use auditor::client::{AuditorClientBuilder, Operator, QueryBuilder};
use auditor::domain::{Component, Record, RecordAdd, RecordDatabase, RecordTest, RecordUpdate};
use chrono::{TimeZone, Utc};
use fake::{Fake, Faker};

#[tokio::test]
async fn add_records() {
    // Arange
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

    let mut saved_records = sqlx::query_as!(
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

    let mut saved_records = sqlx::query_as!(
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

    let mut test_cases: Vec<RecordTest> = (0..100).map(|_| Faker.fake::<RecordTest>()).collect();

    for record in test_cases.iter() {
        let runtime = (record.stop_time.unwrap() - record.start_time.unwrap()).num_seconds();
        let mut transaction = app.db_pool.begin().await.unwrap();

        let id = sqlx::query_unchecked!(
            r#"
                INSERT INTO accounting (
                    record_id, start_time, stop_time, runtime, updated_at
                )
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id;
            "#,
            record.record_id.as_ref(),
            record.start_time,
            record.stop_time,
            runtime,
            Utc::now()
        )
        .fetch_optional(&mut *transaction)
        .await
        .unwrap()
        .unwrap()
        .id;

        for component in record.components.as_ref().unwrap().iter() {
            let (names, scores): (Vec<String>, Vec<f64>) = component
                .scores
                .iter()
                .map(|s| (s.name.as_ref().unwrap().to_string(), s.value.unwrap()))
                .unzip();

            sqlx::query_unchecked!(
                r#"
                WITH insert_components AS (
                    INSERT INTO components (name, amount)
                    VALUES ($1, $2)
                    RETURNING id
                ),
                insert_scores AS (
                    INSERT INTO scores (name, value)
                    SELECT * FROM UNNEST($3::text[], $4::double precision[])
                    -- Update if already in table. This isn't great, but 
                    -- otherwise RETURNING won't return anything.
                    ON CONFLICT (name, value) DO UPDATE
                    SET value = EXCLUDED.value, name = EXCLUDED.name
                    RETURNING id
                ),
                insert_components_scores AS (
                    INSERT INTO components_scores (component_id, score_id)
                    SELECT (SELECT id FROM insert_components), id
                    FROM insert_scores
                )
                INSERT INTO records_components (record_id, component_id)
                SELECT $5, (SELECT id from insert_components) 
                -- FROM accounting WHERE id = $5
                "#,
                component.name.as_ref(),
                component.amount,
                &names[..],
                &scores[..],
                id,
            )
            .execute(&mut *transaction)
            .await
            .unwrap();
        }

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
            .execute(&mut *transaction)
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
    let client = AuditorClientBuilder::new()
        .connection_string(&app.address)
        .build()
        .unwrap();

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

        let id = sqlx::query_unchecked!(
            r#"
                INSERT INTO accounting (
                    record_id, start_time, stop_time, runtime, updated_at
                )
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id;
            "#,
            record.record_id.as_ref(),
            record.start_time,
            record.stop_time,
            runtime,
            Utc::now()
        )
        .fetch_optional(&mut *transaction)
        .await
        .unwrap()
        .unwrap()
        .id;

        for component in record.components.as_ref().unwrap().iter() {
            let (names, scores): (Vec<String>, Vec<f64>) = component
                .scores
                .iter()
                .map(|s| (s.name.as_ref().unwrap().to_string(), s.value.unwrap()))
                .unzip();

            sqlx::query_unchecked!(
                r#"
                WITH insert_components AS (
                    INSERT INTO components (name, amount)
                    VALUES ($1, $2)
                    RETURNING id
                ),
                insert_scores AS (
                    INSERT INTO scores (name, value)
                    SELECT * FROM UNNEST($3::text[], $4::double precision[])
                    -- Update if already in table. This isn't great, but 
                    -- otherwise RETURNING won't return anything.
                    ON CONFLICT (name, value) DO UPDATE
                    SET value = EXCLUDED.value, name = EXCLUDED.name
                    RETURNING id
                ),
                insert_components_scores AS (
                    INSERT INTO components_scores (component_id, score_id)
                    SELECT (SELECT id FROM insert_components), id
                    FROM insert_scores
                )
                INSERT INTO records_components (record_id, component_id)
                SELECT $5, (SELECT id from insert_components) 
                -- FROM accounting WHERE id = $5
                "#,
                component.name.as_ref(),
                component.amount,
                &names[..],
                &scores[..],
                id,
            )
            .execute(&mut *transaction)
            .await
            .unwrap();
        }

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
            .execute(&mut *transaction)
            .await
            .unwrap();
        }

        transaction.commit().await.unwrap();
    }

    let date = Utc.with_ymd_and_hms(2022, 3, 15, 0, 0, 0).unwrap();

    let mut received_records = QueryBuilder::new()
        .with_start_time(Operator::default().gte(date.into()))
        .get(client)
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
    let client = AuditorClientBuilder::new()
        .connection_string(&app.address)
        .build()
        .unwrap();

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

        let id = sqlx::query_unchecked!(
            r#"
                INSERT INTO accounting (
                    record_id, start_time, stop_time, runtime, updated_at
                )
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id;
            "#,
            record.record_id.as_ref(),
            record.start_time,
            record.stop_time,
            runtime,
            Utc::now()
        )
        .fetch_optional(&mut *transaction)
        .await
        .unwrap()
        .unwrap()
        .id;

        for component in record.components.as_ref().unwrap().iter() {
            let (names, scores): (Vec<String>, Vec<f64>) = component
                .scores
                .iter()
                .map(|s| (s.name.as_ref().unwrap().to_string(), s.value.unwrap()))
                .unzip();

            sqlx::query_unchecked!(
                r#"
                WITH insert_components AS (
                    INSERT INTO components (name, amount)
                    VALUES ($1, $2)
                    RETURNING id
                ),
                insert_scores AS (
                    INSERT INTO scores (name, value)
                    SELECT * FROM UNNEST($3::text[], $4::double precision[])
                    -- Update if already in table. This isn't great, but 
                    -- otherwise RETURNING won't return anything.
                    ON CONFLICT (name, value) DO UPDATE
                    SET value = EXCLUDED.value, name = EXCLUDED.name
                    RETURNING id
                ),
                insert_components_scores AS (
                    INSERT INTO components_scores (component_id, score_id)
                    SELECT (SELECT id FROM insert_components), id
                    FROM insert_scores
                )
                INSERT INTO records_components (record_id, component_id)
                SELECT $5, (SELECT id from insert_components) 
                -- FROM accounting WHERE id = $5
                "#,
                component.name.as_ref(),
                component.amount,
                &names[..],
                &scores[..],
                id,
            )
            .execute(&mut *transaction)
            .await
            .unwrap();
        }

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
            .execute(&mut *transaction)
            .await
            .unwrap();
        }

        transaction.commit().await.unwrap();
    }

    let date = Utc.with_ymd_and_hms(2022, 3, 15, 0, 0, 0).unwrap();
    let mut received_records = QueryBuilder::new()
        .with_stop_time(Operator::default().gte(date.into()))
        .get(client)
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
