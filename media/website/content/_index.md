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
cargo install --version=0.5.7 sqlx-cli --no-default-features --features postgres
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
cargo install --version=0.5.7 sqlx-cli --no-default-features --features postgres
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

Auditor is configured via the files in the `configuration` directory or via environment variables as mentioned above.

# Packages

RPMs will be provided in the future.

# SLURM Epilog Collector

The Slurm epilog collector can be built with this command:

```bash
RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl --bin auditor-slurm-epilog-collector
```

The resulting binary can be found in `target/x86_64-unknown-linux-musl/release/auditor-slurm-epilog-collector` and is ideally placed on the Slurm head node.

Add this to your epilog shell script:

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
addr: "auditor_host_addr"
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

todo

# License

Licensed under either of

 - Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/ALU-Schumacher/AUDITOR/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 - MIT License ([LICENSE-MIT](https://github.com/ALU-Schumacher/AUDITOR/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

