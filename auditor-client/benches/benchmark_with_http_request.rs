#![allow(dead_code)]
#![allow(unused_variables)]
use auditor_client::{
    AuditorClient, AuditorClientBuilder, ComponentQuery, MetaOperator, MetaQuery, Operator,
    QueryBuilder,
};
mod configuration;
use crate::configuration::get_configuration;
use auditor::domain::{Component, RecordAdd, RecordTest, Score};
use chrono::naive::Days;
use chrono::{DateTime, TimeZone, Utc};
use fake::{Fake, Faker};
use rand::Rng;
use std::collections::HashMap;

use criterion::{Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;

use rand_distr::Normal;

//#[tokio::main]
async fn insert_records(num: i64, increment: i64) -> Result<(), anyhow::Error> {
    const CHUNK_SIZE: usize = 100;
    let client = start_client()?;

    let mut start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    let mean = 15.0;
    let standard_deviation = 5.0;

    let normal = Normal::new(mean, standard_deviation).unwrap();

    let mut records = Vec::new();

    for i in 0..num {
        let record_id = format!("record-{}", &i);

        let stop_time = start_time.checked_add_signed(chrono::Duration::hours(
            generate_stop_time_duration(&mut rand::thread_rng(), &normal),
        ));

        let mut meta = HashMap::new();
        let meta1 = generate_site_id(&mut rand::thread_rng());
        let meta3 = generate_site_id(&mut rand::thread_rng());
        let meta2 = generate_group_id(&mut rand::thread_rng());
        meta.insert("site_id".to_string(), vec![meta1]);
        meta.insert("group_id".to_string(), vec![meta2]);

        let score: i64 = generate_component_scores(&mut rand::thread_rng());

        let component_cpu = Component::new("CPU", score)?.with_score(Score::new("HEPSPEC06", 9.2)?);

        let component_mem = Component::new("MEM", 32)?;

        let components = vec![component_cpu, component_mem];

        let record = RecordAdd::new(record_id, meta, components, start_time)
            .expect("Could not construct record")
            .with_stop_time(stop_time.unwrap());

        records.push(record);

        if records.len() >= CHUNK_SIZE {
            client.bulk_insert(&records).await?;
            records.clear();
        }

        start_time = start_time
            .checked_add_signed(chrono::Duration::seconds(increment))
            .unwrap();
    }

    Ok(())
}

fn start_client() -> Result<AuditorClient, anyhow::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");

    println!("{} {}", &configuration.addr, configuration.port.clone());

    let client = AuditorClientBuilder::new()
        .address(&configuration.addr, configuration.port)
        .timeout(20)
        .build()?;

    Ok(client)
}

async fn insert_worst_case_records(num: i64) -> Result<(), anyhow::Error> {
    let client = start_client()?;

    let records: Vec<RecordTest> = (0..num).map(|_| Faker.fake()).collect();

    let chunk_size = 1000;
    let chunks = records.chunks(chunk_size);

    for chunk in chunks {
        let test_cases: Vec<RecordAdd> = chunk
            .iter()
            .cloned()
            .map(RecordAdd::try_from)
            .map(Result::unwrap)
            .collect();

        client.bulk_insert(&test_cases).await?;
    }

    Ok(())
}

fn generate_group_id(rng: &mut impl Rng) -> String {
    let random_group_id = rng.gen_range(1..=6);
    format!("group_{random_group_id}")
}

fn generate_site_id(rng: &mut impl Rng) -> String {
    let random_group_id = rng.gen_range(1..=6);
    format!("site_{random_group_id}")
}

fn generate_component_scores(rng: &mut impl Rng) -> i64 {
    let random_component_score: i64 = rng.gen_range(5..=10);
    random_component_score
}

fn generate_stop_time_duration<R: Rng>(rng: &mut R, normal: &Normal<f64>) -> i64 {
    let random_stop_time_duration: i64 = rng.sample(normal).round() as i64;
    random_stop_time_duration.abs()
}

fn benchmark_record_query_from_auditor(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark_record_query_from_auditor");

    let configuration = get_configuration().expect("Failed to read configuration.");

    let rt = Runtime::new().unwrap();

    let client = start_client().unwrap();

    group.sample_size(configuration.sample_size);

    let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    let num: i64 = configuration.num_of_records;

    let increment: i64 = 315i64;

    let test = rt.block_on(async { insert_records(num, increment).await });

    rt.block_on(async {
        let response = client.health_check().await;

        assert!(response)
    });

    rt.block_on(async {
        let check_if_all_records_exists =
            QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));
        let response = check_if_all_records_exists.get(client.clone()).await;
        assert!(response.unwrap().len() == 10000)
    });

    let get_all_records =
        QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));

    group.bench_function("queyring_all_records", |b| {
        b.iter(|| rt.block_on(async { get_all_records.clone().get(client.clone()).await }))
    });

    let query_start_time = start_time.checked_add_days(Days::new(100)).unwrap();

    let stop_time = query_start_time.checked_add_days(Days::new(120)).unwrap();

    let time_range_query = QueryBuilder::new().with_start_time(
        Operator::default()
            .gte(start_time.into())
            .lt(stop_time.into()),
    );

    group.bench_function("querying_within_a_time_range", |b| {
        b.iter(|| rt.block_on(async { time_range_query.clone().get(client.clone()).await }))
    });

    let time_and_site_query = QueryBuilder::new()
        .with_meta_query(MetaQuery::new().meta_operator(
            "site_id".to_string(),
            MetaOperator::default().contains(vec!["site_1".to_string()]),
        ))
        .with_start_time(
            Operator::default()
                .gte(start_time.into())
                .lt(stop_time.into()),
        );

    group.bench_function("querying_one_site_id_within_a_time_range", |b| {
        b.iter(|| rt.block_on(async { time_and_site_query.clone().get(client.clone()).await }))
    });

    let count: u8 = 6u8;
    let component_query = QueryBuilder::new().with_component_query(
        ComponentQuery::new()
            .component_operator("cpu".to_string(), Operator::default().equals(count.into())),
    );

    group.bench_function("querying_records_with_component_name", |b| {
        b.iter(|| rt.block_on(async { component_query.clone().get(client.clone()).await }))
    });

    let time_meta_component_query = QueryBuilder::new()
        .with_start_time(
            Operator::default()
                .gte(start_time.into())
                .lt(stop_time.into()),
        )
        .with_meta_query(MetaQuery::new().meta_operator(
            "site_id".to_string(),
            MetaOperator::default().contains(vec!["site_1".to_string()]),
        ))
        .with_component_query(
            ComponentQuery::new()
                .component_operator("cpu".to_string(), Operator::default().equals(count.into())),
        );

    group.bench_function("querying_with_time_meta_component_fields", |b| {
        b.iter(|| {
            rt.block_on(async { time_meta_component_query.clone().get(client.clone()).await })
        })
    });

    let time_two_meta_and_component_query = QueryBuilder::new()
        .with_start_time(
            Operator::default()
                .gte(start_time.into())
                .lt(stop_time.into()),
        )
        .with_meta_query(MetaQuery::new().meta_operator(
            "group_id".to_string(),
            MetaOperator::default().contains(vec!["group_1".to_string()]),
        ))
        .with_component_query(
            ComponentQuery::new()
                .component_operator("cpu".to_string(), Operator::default().equals(count.into())),
        );

    group.bench_function("querying_with_time_two_meta_and_component_fields", |b| {
        b.iter(|| {
            rt.block_on(async {
                time_two_meta_and_component_query
                    .clone()
                    .get(client.clone())
                    .await
            })
        })
    });
}

// Change here to specify the function you would like to benchmark. Please specify only one
// function name at a time.
criterion_group!(benches, benchmark_record_query_from_auditor);
criterion_main!(benches);
