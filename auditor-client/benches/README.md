# Benchmarking Auditor Performance

The Auditor benchmark assesses the read performance of the main AUDITOR component, which stores records in POSTGRES. This benchmark evaluates the efficiency of various filters and queries for retrieving record information. Criterion is employed to conduct the benchmarking, and the results are stored in JSON format.

## How to Run the Benchmark

1. Start a new instance of AUDITOR and note the address and port of the AUDITOR client endpoint.
2. Specify the address and port information in `bench.yaml`, located in `auditor/benches/configuration`.
3. Execute the command `cargo bench` from the home directory of AUDITOR.

After completing the benchmark, you can visualize the results in the web version located at `target/criterion/report/index.html`. Alternatively, navigate to a subfolder and open `report/index.html` to view specific query performance results.

### Understanding Benchmarking Process

The benchmarking process involves two main steps:

1. **Inserting Records**: Records are inserted into the database based on the desired size for benchmarking.
  
2. **Executing Benchmarks**: The `benchmark_with_http_request.rs` file contains the logic for benchmarking. The functions to be benchmarked are specified as parameters to the `criterion_group!` macro call, like so: ```criterion_group!(benches, real_case_record_size_10_000);```


You can modify this call to benchmark different functions as needed.
