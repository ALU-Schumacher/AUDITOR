use auditor::client::{
    AuditorClient, AuditorClientBuilder, ComponentQuery, MetaOperator, MetaQuery, Operator,
    QueryBuilder,
};
mod configuration;
use crate::configuration::get_configuration;
use auditor::domain::{RecordAdd, RecordTest, Score, Component};
use chrono::{DateTime, Utc, TimeZone};
use chrono::naive::Days;
use fake::{Fake, Faker};
use rand::Rng;
use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;
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
            let meta2 = generate_group_id(&mut rand::thread_rng());
            meta.insert("site_id".to_string(), vec![meta1]);
            meta.insert("group_id".to_string(), vec![meta2]);
            
            let score: i64 = generate_component_scores(&mut rand::thread_rng());

            let component_cpu = Component::new("CPU", score)?
                .with_score(Score::new("HEPSPEC06", 9.2)?);

            // Create a second component (32 GB memory)
            let component_mem = Component::new("MEM", 32)?;

            // Store components in a vector
            let components = vec![component_cpu, component_mem];


            let record = RecordAdd::new(record_id, meta, components, start_time).expect("Could not construct record").with_stop_time(stop_time.unwrap());

            records.push(record);
        
            if records.len() >= CHUNK_SIZE {
                // Bulk insert current chunk
                client.bulk_insert(&records).await?;
                records.clear();
             }

            start_time =
                start_time.checked_add_signed(chrono::Duration::seconds(increment)).unwrap();

        }

    Ok(())
}

fn start_client() -> Result<AuditorClient, anyhow::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");

    println!("{} {}", &configuration.addr, configuration.port.clone());

    let client = AuditorClientBuilder::new()
        .address(
            &configuration.addr,
            configuration.port,
        )
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
    format!("group_{}", random_group_id)
}

fn generate_site_id(rng: &mut impl Rng) -> String {
    let random_group_id = rng.gen_range(1..=6);
    format!("site_{}", random_group_id)
}

fn generate_component_scores(rng: &mut impl Rng) -> i64 {
    let random_component_score: i64 = rng.gen_range(5..=10);
    random_component_score
}

fn generate_stop_time_duration<R: Rng>(rng: &mut R, normal: &Normal<f64>) -> i64 {
    let random_stop_time_duration: i64 = rng.sample(normal).round() as i64;
    random_stop_time_duration.abs()
}

fn worst_case_insertion_benchmark_with_client(c: &mut Criterion) {
    let mut group = c.benchmark_group("Insertion of records");

    let rt = Runtime::new().unwrap();

    let client = start_client().unwrap();

    group.measurement_time(Duration::from_secs(30));

    group.sample_size(100);

    group.bench_function("inserting_1000_records", |b| {
        let records: Vec<RecordTest> = (0..1000).map(|_| Faker.fake()).collect();

        let test_cases: Vec<RecordAdd> = records
            .iter()
            .cloned()
            .map(RecordAdd::try_from)
            .map(Result::unwrap)
            .collect();

        b.iter(|| rt.block_on(async { client.bulk_insert(black_box(&test_cases)).await }))
    });

    group.bench_function("inserting_100_records", |b| {
        let records: Vec<RecordTest> = (0..100).map(|_| Faker.fake()).collect();

        let test_cases: Vec<RecordAdd> = records
            .iter()
            .cloned()
            .map(RecordAdd::try_from)
            .map(Result::unwrap)
            .collect();

        b.iter(|| rt.block_on(async { client.bulk_insert(black_box(&test_cases)).await }))
    });

    group.finish();
}

fn real_case_record_size_100_000(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_case_query_record_size_100_000");

    let rt = Runtime::new().unwrap();

    let client = start_client().unwrap();

    group.sample_size(3000);

    let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    let num: i64 = 100_000i64;

    let increment: i64 = 315i64;

    let _ = rt.block_on(async { insert_records(num, increment).await });

    rt.block_on( async { 
        let response = client.health_check().await;

        assert!(response == true)
    });

    rt.block_on( async { 
        let check_if_all_records_exists = QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));
        let response = check_if_all_records_exists.get(client.clone()).await;
        assert!(response.unwrap().len() == 100_000)
    });


    let get_all_records =
        QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));

    group.bench_function("queyring_all_100_000_records", |b| {
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
            MetaOperator::default().contains("site_1".to_string()),
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
            MetaOperator::default().contains("site_1".to_string()),
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
            MetaOperator::default().contains("group_1".to_string()),
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

fn real_case_record_size_1_000_000(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_case_query_record_size_1_000_000");

    let rt = Runtime::new().unwrap();

    let client = start_client().unwrap();

    group.sample_size(3000);

    let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    let num: i64 = 100_000i64;

    let increment: i64 = 31i64;

    let _ = rt.block_on(async { insert_records(num, increment).await });
    
    rt.block_on( async { 
        let response = client.health_check().await;

        assert!(response == true)
    });


    rt.block_on( async { 
        let check_if_all_records_exists = QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));
        let response = check_if_all_records_exists.clone().get(client.clone()).await;

        assert!(response.unwrap().len() == 1_000_000)
    });

    let get_all_records =
        QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));

    group.bench_function("queyring_started_since_from_1_000_000_records", |b| {
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
            MetaOperator::default().contains("site_1".to_string()),
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
            MetaOperator::default().contains("site_1".to_string()),
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
            MetaOperator::default().contains("group_1".to_string()),
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

fn real_case_record_size_10_000_000(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_case_query_record_size_10_000_000");

    let rt = Runtime::new().unwrap();

    let client = start_client().unwrap();

    group.sample_size(3000);

    let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    let num: i64 = 10_000_000i64;

    let increment: i64 = 3i64;

    let _ = rt.block_on(async { insert_records(num, increment).await });

    rt.block_on( async { 
        let response = client.health_check().await;

        assert!(response == true)
    });

    rt.block_on( async { 
        let check_if_all_records_exists = QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));
        let response = check_if_all_records_exists.clone().get(client.clone()).await;

        assert!(response.unwrap().len() == 100_000)
    });


    let get_all_records =
        QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));

    group.bench_function("queyring_started_since_10_000_000_records", |b| {
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
            MetaOperator::default().contains("site_1".to_string()),
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
            MetaOperator::default().contains("site_1".to_string()),
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
            MetaOperator::default().contains("group_1".to_string()),
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

fn worst_case_record_size_100_000(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case_query_record_size_100_000");

    let rt = Runtime::new().unwrap();

    let client = start_client().unwrap();
    let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    let num: i64 = 100_000i64;

    let _ = rt.block_on(async { insert_worst_case_records(num).await });

    let get_all_records =
        QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));
    
    rt.block_on( async { 
        let response = client.health_check().await;

        assert!(response == true)
    });

    rt.block_on( async { 
        let check_if_all_records_exists = QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));
        let response = check_if_all_records_exists.clone().get(client.clone()).await;

        assert!(response.unwrap().len() == 100_000)
    });

    group.bench_function("queyring_started_since_from_100_000_records", |b| {
        b.iter(|| rt.block_on(async { get_all_records.clone().get(client.clone()).await }))
    });

    let stop_time = start_time.clone().checked_add_days(Days::new(100));

    let record_site_query = QueryBuilder::new()
        .with_meta_query(MetaQuery::new().meta_operator(
            "site_id".to_string(),
            MetaOperator::default().contains("group_1".to_string()),
        ))
        .with_start_time(
            Operator::default()
                .gte(start_time.into())
                .lt(stop_time.unwrap().into()),
        );

    group.bench_function("querying_one_site_id_within_a_time_range", |b| {
        b.iter(|| rt.block_on(async { record_site_query.clone().get(client.clone()).await }))
    });

    let time_range_query = QueryBuilder::new().with_start_time(
        Operator::default()
            .gte(start_time.into())
            .lt(stop_time.unwrap().into()),
    );

    group.bench_function("querying_within_a_time_range", |b| {
        b.iter(|| rt.block_on(async { time_range_query.clone().get(client.clone()).await }))
    });

    let count: u8 = 6u8;
    let component_query = QueryBuilder::new().with_component_query(
        ComponentQuery::new()
            .component_operator("cpu".to_string(), Operator::default().equals(count.into())),
    );

    group.bench_function("querying_records_with_component_name", |b| {
        b.iter(|| rt.block_on(async { component_query.clone().get(client.clone()).await }))
    });

    let filter_on_time_meta_components = QueryBuilder::new()
        .with_start_time(
            Operator::default()
                .gte(start_time.into())
                .lt(stop_time.unwrap().into()),
        )
        .with_meta_query(MetaQuery::new().meta_operator(
            "site_id".to_string(),
            MetaOperator::default().contains("group_1".to_string()),
        ))
        .with_component_query(
            ComponentQuery::new()
                .component_operator("cpu".to_string(), Operator::default().equals(count.into())),
        );

    group.bench_function("querying_with_time_meta_component_fields", |b| {
        b.iter(|| {
            rt.block_on(async {
                filter_on_time_meta_components
                    .clone()
                    .get(client.clone())
                    .await
            })
        })
    });
}

fn worst_case_record_size_1_000_000(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case_query_record_size_100_000");

    let rt = Runtime::new().unwrap();

    let client = start_client().unwrap();
    let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    let num: i64 = 1_000_000i64;

    let _ = rt.block_on(async { insert_worst_case_records(num).await });
    
    rt.block_on( async { 
        let response = client.health_check().await;

        assert!(response == true)
    });

    rt.block_on( async { 
        let check_if_all_records_exists = QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));
        let response = check_if_all_records_exists.clone().get(client.clone()).await;

        assert!(response.unwrap().len() == 100_000)
    });

    let get_all_records =
        QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));

    group.bench_function("queyring_started_since_from_100_000_records", |b| {
        b.iter(|| rt.block_on(async { get_all_records.clone().get(client.clone()).await }))
    });

    let stop_time = start_time.clone().checked_add_days(Days::new(100));

    let record_site_query = QueryBuilder::new()
        .with_meta_query(MetaQuery::new().meta_operator(
            "site_id".to_string(),
            MetaOperator::default().contains("group_1".to_string()),
        ))
        .with_start_time(
            Operator::default()
                .gte(start_time.into())
                .lt(stop_time.unwrap().into()),
        );

    group.bench_function("querying_one_site_id_within_a_time_range", |b| {
        b.iter(|| rt.block_on(async { record_site_query.clone().get(client.clone()).await }))
    });

    let time_range_query = QueryBuilder::new().with_start_time(
        Operator::default()
            .gte(start_time.into())
            .lt(stop_time.unwrap().into()),
    );

    group.bench_function("querying_within_a_time_range", |b| {
        b.iter(|| rt.block_on(async { time_range_query.clone().get(client.clone()).await }))
    });

    let count: u8 = 6u8;
    let component_query = QueryBuilder::new().with_component_query(
        ComponentQuery::new()
            .component_operator("cpu".to_string(), Operator::default().equals(count.into())),
    );

    group.bench_function("querying_records_with_component_name", |b| {
        b.iter(|| rt.block_on(async { component_query.clone().get(client.clone()).await }))
    });

    let filter_on_time_meta_components = QueryBuilder::new()
        .with_start_time(
            Operator::default()
                .gte(start_time.into())
                .lt(stop_time.unwrap().into()),
        )
        .with_meta_query(MetaQuery::new().meta_operator(
            "group_id".to_string(),
            MetaOperator::default().contains("group_1".to_string()),
        ))
        .with_component_query(
            ComponentQuery::new()
                .component_operator("cpu".to_string(), Operator::default().equals(count.into())),
        );

    group.bench_function("querying_with_time_meta_component_fields", |b| {
        b.iter(|| {
            rt.block_on(async {
                filter_on_time_meta_components
                    .clone()
                    .get(client.clone())
                    .await
            })
        })
    });
}

fn worst_case_record_size_10_000_000(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case_query_record_size_100_000");

    let rt = Runtime::new().unwrap();

    let client = start_client().unwrap();

    group.measurement_time(Duration::from_secs(30));

    let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    let num: i64 = 10_000_000i64;

    let _ = rt.block_on(async { insert_worst_case_records(num).await });
    
    rt.block_on( async { 
        let response = client.health_check().await;

        assert!(response == true)
    });

    rt.block_on( async { 
        let check_if_all_records_exists = QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));
        let response = check_if_all_records_exists.clone().get(client.clone()).await;

        assert!(response.unwrap().len() == 100_000)
    });

    let get_all_records =
        QueryBuilder::new().with_start_time(Operator::default().gte(start_time.into()));

    group.bench_function("queyring_started_since_from_100_000_records", |b| {
        b.iter(|| rt.block_on(async { get_all_records.clone().get(client.clone()).await }))
    });

    let stop_time = start_time.clone().checked_add_days(Days::new(100));

    let record_site_query = QueryBuilder::new()
        .with_meta_query(MetaQuery::new().meta_operator(
            "group_id".to_string(),
            MetaOperator::default().contains("group_1".to_string()),
        ))
        .with_start_time(
            Operator::default()
                .gte(start_time.into())
                .lt(stop_time.unwrap().into()),
        );

    group.bench_function("querying_one_site_id_within_a_time_range", |b| {
        b.iter(|| rt.block_on(async { record_site_query.clone().get(client.clone()).await }))
    });

    let time_range_query = QueryBuilder::new().with_start_time(
        Operator::default()
            .gte(start_time.into())
            .lt(stop_time.unwrap().into()),
    );

    group.bench_function("querying_within_a_time_range", |b| {
        b.iter(|| rt.block_on(async { time_range_query.clone().get(client.clone()).await }))
    });

    let count: u8 = 6u8;
    let component_query = QueryBuilder::new().with_component_query(
        ComponentQuery::new()
            .component_operator("cpu".to_string(), Operator::default().equals(count.into())),
    );

    group.bench_function("querying_records_with_component_name", |b| {
        b.iter(|| rt.block_on(async { component_query.clone().get(client.clone()).await }))
    });

    let filter_on_time_meta_components = QueryBuilder::new()
        .with_start_time(
            Operator::default()
                .gte(start_time.into())
                .lt(stop_time.unwrap().into()),
        )
        .with_meta_query(MetaQuery::new().meta_operator(
            "group_id".to_string(),
            MetaOperator::default().contains("group_1".to_string()),
        ))
        .with_component_query(
            ComponentQuery::new()
                .component_operator("cpu".to_string(), Operator::default().equals(count.into())),
        );

    group.bench_function("querying_with_time_meta_component_fields", |b| {
        b.iter(|| {
            rt.block_on(async {
                filter_on_time_meta_components
                    .clone()
                    .get(client.clone())
                    .await
            })
        })
    });
}

criterion_group!(benches, real_case_record_size_100_000);
criterion_main!(benches);
