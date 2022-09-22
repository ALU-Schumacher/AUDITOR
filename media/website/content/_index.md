+++
title = "Auditor"
sort_by = "weight"
+++

# Auditor

Auditor stands for **A**cco**u**nting **D**ata Handl**i**ng **T**oolbox For **O**pportunistic **R**esources. 
Auditor ingests accounting data provided by so-called *collectors*, stores it and provides it to the outside to so-called *plugins*.

It comes with a well-defined REST API which allows for the implementation of application-specific collectors and plugins. This makes it well suited for a wide range of use cases.

TODO: Grafik, detaillierte Erklaerung.

## Features

* TODO

# Running Auditor

Auditor can be run by compiling the source from the repository or by running a pre-built docker container.
Both methods require that the PostgreSQL database is installed migrated beforehand. 

## Migrating the database

Migrating the database requires cloning the Auditor repository and installing `cargo` and `sqlx`.
A prerequisite is a working Rust setup, which can be installed either via your distributions package manager or via the following command:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Now `sqlx` can be installed via `cargo`:

```bash
cargo install --version=0.6.2 sqlx-cli --no-default-features --features postgres,rustls
```

Clone the repository and `cd` into the directory.

```bash
git clone git@github.com:ALU-Schumacher/AUDITOR.git 
cd AUDITOR
```

To migrate the database, run the following from the root directory of the repo:

```bash
# Adapt these variables to your setup
DB_USER="postgres"
DB_PASSWORD="password"
DB_NAME="auditor"
DB_HOST="localhost"
DB_PORT="5432"

export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
sqlx database create
sqlx migrate run
```

This assumes that the PostgreSQL database is already installed.

## Using Docker

The easiest way to run Auditor is via a Docker container from [Docker Hub](https://hub.docker.com/repository/docker/aluschumacher/auditor) or [Github Container Registry](https://github.com/ALU-Schumacher/AUDITOR/pkgs/container/auditor).
Auditor requires a properly configured PostgreSQL database.
After installing PostgreSQL, the database needs to be migrated with `sqlx`.

AUDITORs configuration can be adapted with environment variables.

| Variable                          | Description                                               |
| --------                          | -----------                                               |
| `AUDITOR_APPLICATION__ADDR`       | Address to bind to (default `0.0.0.0`)                    |
| `AUDITOR_APPLICATION__PORT`       | Port to bind to (default `8000`)                          |
| `AUDITOR_DATABASE__HOST`          | Host address of PostgreSQL database (default `localhost`) |
| `AUDITOR_DATABASE__PORT`          | Port of PostgreSQL database (default `5432`)              |
| `AUDITOR_DATABASE__USERNAME`      | PostgreSQL database username (default `postgres`)         |
| `AUDITOR_DATABASE__PASSWORD`      | PostgreSQL database password (default `password`)         |
| `AUDITOR_DATABASE__REQUIRE_SSL`   | Whether or not to use SSL (default `true`)                |

Use `docker run` to execute Auditor:

```bash
docker run aluschumacher/auditor:main
```

The configuration parameters can be set by passing environment variables via `-e`:

```bash
docker run -e AUDITOR_APPLICATION__ADDR=localhost -e AUDITOR_DATABASE__REQUIRE_SSL=false aluschumacher/auditor:main
```

## Configuration files

Besides environment variables, a YAML configuration file can be used:

```yaml
application:
  addr: 0.0.0.0
  port: 8000
database:
  host: "localhost"
  port: 5432
  username: "postgres"
  password: "password"
  database_name: "auditor"
  require_ssl: false
metrics:
  database:
    frequency: 30
    metrics:
      - RecordCount
      - RecordCountPerSite
      - RecordCountPerGroup
      - RecordCountPerUser
```

This configuration file can be passed to Auditor and will overwrite the default configuration.

## Metrics exporter for Prometheus

Metrics for Prometheus are exposed via the `/metrics` endpoint.
By default HTTP metrics are exported.
In addition, database metrics are exported as well (optional).
These include the current number of records in the database, as a well as the number of records per site, group and user.
Database metrics export can be configured in the configuration:

```yaml
metrics:
  database:
    # How often thes values are computed (default: every 30 seconds)
    frequency: 30
    # Type of metrics to export (default: None)
    metrics:
      - RecordCount
      - RecordCountPerSite
      - RecordCountPerGroup
      - RecordCountPerUser
```

How often the database metrics are computed is defined by the `frequency` configuration variable. 
Note that computing the database metrics is a potentially expensive operation.
Therefore it is advised to monitor the performance of Auditor when working with databases with a large number of records.
The frequency setting should be somewhat in accordance with the Prometheus scraping interval.


# Compiling Auditor

Alternatively, Auditor can be compiled and run directly.
This section descibes how to set up the required development environment.
It also covers cross compiling and static linking against `musl` instead of `glibc` in order to obtain a maximally portable binary.

## Prerequisites

Compiling Auditor requires

* Rust
* Docker
* sqlx-cli
* PostgreSQL
* bunyan (optional)


### Rust

Requires a recent Rust version (MSRV 1.56.1) and cargo.

Ideally installed via rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### sqlx

```bash
cargo install --version=0.6.2 sqlx-cli --no-default-features --features postgres,rustls
```

### bunyan

For nicer logs install bunyan:

```bash
cargo install bunyan
```


## Running Auditor via cargo

```bash
git clone git@github.com:ALU-Schumacher/AUDITOR.git 
cd AUDITOR
./scripts/db_init.sh
cargo run
```

Calling `./scripts/db_init.sh` will start a temporary PostgreSQL database in a Docker container and will automatically migrate the database.
If you plan to run Auditor like this in production, make sure to properly set up a database and instead of calling `db_init.sh` pass the correct settings to auditor via the configuration environement variables mentioned above.
Building requires a running database, because database queries are checked against the database during runtime! This can be disabled with:

```bash
SQLX_OFFLINE=true cargo run
```

For nicer logs pipe the output through `bunyan`:

```bash
cargo run | bunyan
```

Note that this will be a debug build which will not pass all optimizations during compilation. 
For maximum performance a `release` build is necessary:

```bash
cargo run --release
```

## Running the tests

```bash
cargo test
```

Output logs while running the tests:

```bash
TEST_LOG=true cargo test 
```

## Building binaries

Binaries used in production should be built in release mode:

```bash
cargo build --release
```

The binary can then be found in `target/release/auditor`.

Make sure a database is up and running when starting Auditor.

## Static linking and cross compiling

The binary will only link to the system `glibc`.
Fully statically linked binaries can be obtained by statically linking against `musl` instead auf `glibc`.
This can be beneficial when cross compiling, in particular for older targets (e.g. CentOS7).
This requires the `musl` dev tools to be installed on the system where Auditor is compiled.
The exact packages needed depends on your Linux distribtion.
Furthermore, the `x86_64-unknown-linux-musl` target for Rust must be installed (once):

```bash
rustup target add x86_64-unknown-linux-musl
```

Then cross-compilation with static linking can be done via

```bash
RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl
```

The resulting binaries will be placed in `target/x86_64-unknown-linux-musl/release`.

## Configuration

Auditor is configured via the files in the `configuration` directory, a configuration file passed to the binary, or via environment variables as mentioned above.


# Packages

RPMs are provided for each release on the [Github release page](https://github.com/ALU-Schumacher/AUDITOR/releases).

# SLURM Epilog Collector

The Slurm epilog collector can installed from the provided RPM or can be built with this command:

```bash
RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl --bin auditor-slurm-epilog-collector
```

The resulting binary can be found in `target/x86_64-unknown-linux-musl/release/auditor-slurm-epilog-collector` and is ideally placed on the Slurm head node.

Add this to your epilog shell script (on the slurm head node):

```bash
#!/bin/sh

# Divert stdout and sterr. Make sure the slurm user has write access to both locations.
# Ideally there is also log rotation in place for those logs.
exec >> /epilog_logs/epilog.log
exec 2>> /epilog_logs/epilog.log

/absolute/path/to/auditor-slurm-epilog-collector /absolute/path/to/auditor-slurm-epilog-collector-config.yaml
```

This will read the `$SLURM_JOB_ID` environment variable, which is only available in the context of a SLURM epilog script.

Internally, `scontrol` is called to obtain the necessary information of the job. 

If not all jobs are of relevance, filtering should be done in the epilog script such that the collector is only executed for relevant jobs.
This avoids unnecessary and potentially expensive calls to `scontrol`.
Slurm provides a number of environment variables in the context of an epilog script which are listed in the [Slurm documentation](https://slurm.schedmd.com/prolog_epilog.html).

Example:

```bash
#!/bin/sh

# Only execute collector for jobs running on `some_partition`
if [ "$SLURM_JOB_PARTITION" == "some_partition" ]; then
	LOG_FILE=/path/to/epilog.log
	exec >> $LOG_FILE
	exec 2>> $LOG_FILE

	/absolute/path/to/auditor-slurm-epilog-collector /absolute/path/to/auditor-slurm-epilog-collector-config.yaml
fi
```

## Example configurations

The following configuration shows how to set the Auditor host address and port.
The `record_prefix` will be used to prefix the Slurm job id in the record identifier (in this case it will be `slurm-JOBID`).
The `site_name` is the `site_id` which will be attached to every record.
`components` defines how to extract accountable information from the call to `scontrol` and attaches `score`s to it. 
In the context of `components`, `name` indicates how this component will be identified in the final record and `key` indicates the `key` which is to be extracted from the `scontrol` output.
`scores` are optional.

```yaml
addr: "auditor_host_addr"
port: 8000
record_prefix: "slurm"
site_id: "site_name"
components:
  - name: "Cores"
    key: "NumCPUs"
    scores:
      - name: "HEPSPEC"
        factor: 1.0
  - name: "Memory"
    key: "Mem"
```

Extraction of components as well as adding of scores can be done conditionally, as shown in the following example configuration.
The matching is performed on values associated with certain keys in the `scontrol` output.
Regex is accepted.

```yaml
addr: "auditor_host_address"
port: 8000
record_prefix: "slurm"
site_id: "site_name"
components:
  - name: "Cores"
    key: "NumCPUs"
    scores:
      # If it the job is running on partition `part1`, then use HEPSPEC value 1.1
      - name: "HEPSPEC"
        factor: 1.1
        only_if:
          key: "Partition"
          matches: "^part1$"
      # If it the job is running on partition `part2`, then use HEPSPEC value 1.2
      - name: "HEPSPEC"
        factor: 1.2
        only_if:
          key: "Partition"
          matches: "^part2$"
  - name: "Memory"
    key: "Mem"
    only_if:
      key: "Partition"
      matches: "^part2$"
```

# Priority Plugin

The priority plugin takes the resources provided by multiple groups and computes a priority for each of these groups based on who many resources wer provided.
This allows one to transfer provided resources on one system to priorities in another system.
The computed priorities are set via shelling out, and the executed commands can be defined as needed.

The priority plugin is available as RPM or can be built with the command:

```bash
RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl --bin auditor-priority-plugin
```

The resulting binary can be found in `target/x86_64-unknown-linux-musl/release/auditor-priority-plugin` and is ideally placed on a node where the priorities should be set.

One run of the priority plugin will update the priorities once.
In general, the priority plugin should be run on a regular basis (for instance as a cron job), depending on how often the update should be performed.

A typical configuration for the SLURM batch system may look like this:

```yaml
addr: "auditor_host_address"
port: 8000
duration: 1209600 # in seconds
components:
  NumCPUs: "HEPSPEC"
group_mapping:
  group1: 
    - "part1"
  group2:
    - "part2"
  group3:
    - "part3"
command:
  - "/usr/bin/scontrol update PartitionName={1} PriorityJobFactor={priority}"
min_priority: 1
max_priority: 65335
computation_mode: ScaledBySum
```

The resources used for calculating the priorities can be configured via the `components` field.
It defines which components to extract from the `components` field of the record (`NumCPUs` in this example), as well as the corresponding score (`HEPSPEC` in this example).
Multiple components can be extracted. 
The configured components and scores must be part of the records.
The resources of each component will be multiplied by the corresponding score and the resulting provided resource per group is the sum of all these.
The records considered in the computation can be limited to all records which finished in the past X seconds via the `duration` field (in seconds).
Omitting this field takes all records in the database into account.
Via the `group_mapping` field, it is possible to attach certain additional information to the individual groups which are to be considered in the calculation.
In the example configuration above are three groups `group{1,2,3}`, where each has a corresponding partition `part{1,2.3}`.
These mappings can be accessed when constructing the `command`s which will be executed after computing the priorities by using `{N}` where `N` corresponds to number of the element in the list of the `group_mapping`.
For instance, for `group1`, the string `{1}` indicates `part1` while for `group2` the same string `{1}` indicates `part2`.
The `command` field in the configuration illustrates the usage of these mappings.
This allows one to adapt the command for the various groups involved.
In the `command` field one can also see a string `{priority}`, which will be replaced by the computed priority for the group.
Another special string, `{resources}` is available, which is replaced by the computed provided resource per group.
The command is executed for each group separately and multiple commands can be provided.

## Priority computation modes

As stated above, the priorities are computed from the provided resources of each group.
However, the computed resources and the priorities are in different units and span different ranges.
Therefore a mapping between resources and priorities needs to be in place.
This plugin offers two `computation_modes`: `FullSpread` and `ScaledBySum`.
Via `min_priority` and `max_priority`, lower and upper limits on the computed priority are set.

* `FullSpread`:  This mode will spread the resources on the full range given by `min_priority` and `max_priority`, such that the group with the least provided resources will be assigned a priority equal to `min_priority` and the group with the most provided resources will be assigned a priority equal to `max_priority`. All other groups are distributed inside that range according to their provided resources. This creates maximum spread of the priorities. A disadvantage of this approach is that the computed priorites of two consecutive runs can be substantially different, leading to large jumps in priorities.
* `ScaledBySum`: Computes the priorities such that `max_priority` is equal to the sum of all provide resources plus `min_priority`. This leads to a smoother change of priorities over multiple runs of the plugin. The maximum priority can only be reached by a group if all other groups provide no resources. 

# Auditor Clients

To facilitate the development of collectors and plugins, client libraries for Rust and Python are offered which handle the interaction with the Auditor server.
For details please consult the respective documentation pages for [the Rust client](https://docs.rs/auditor/) and [the Python client](https://ALU-Schumacher.github.io/AUDITOR/pyauditor/).

# License

Licensed under either of

 - Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/ALU-Schumacher/AUDITOR/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 - MIT License ([LICENSE-MIT](https://github.com/ALU-Schumacher/AUDITOR/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

