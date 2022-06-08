# AUDITOR

## Prerequisites

Requires

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

## Usage

```bash
git clone git@github.com:ALU-Schumacher/AUDITOR.git 
cd AUDITOR
./scripts/db_init.sh
cargo run
```

For nicer logs run AUDITOR like this:

```bash
cargo run | bunyan
```

## Running the tests

```bash
cargo test
```

Running the test with output of logs:

```bash
TEST_LOG=true cargo test 
```

## Building binaries

Binaries used in production should be built in release mode:

```bash
cargo build --release
```

The binary can then be found in `target/release/auditor`.

Make sure a database is up and running when starting AUDITOR.

## Configuration

AUDITOR is configured via the files in the `configuration` directory.

## Running in Docker

AUDITOR can be run in a Docker container from [Docker Hub](https://hub.docker.com/repository/docker/aluschumacher/auditor) or [Github Container Registry](https://github.com/ALU-Schumacher/AUDITOR/pkgs/container/auditor).
When running in a container, a PostgreSQL database needs to be set up and configured manually.
After installing PostgreSQL, the database needs to be migrated with `sqlx`.

To do so, run the following from the root directory of the repo:
```bash
# Adapt these variables to your setup
DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB:=auditor}"
DB_PORT="${POSTGRES_PORT:=5432}"

export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
sqlx database create
sqlx migrate run
```

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


## SLURM Epilog Collector

Directly calling the binary:

```bash
# Divert stdout and sterr. Make sure the slurmuser has write access to both locations.
exec >> /epilog_logs/epilog.log
exec 2>> /epilog_logs/epilog.log

AUDITOR_ADDR=auditor_host_address AUDITOR_PORT=8000 /auditor-slurm-epilog-collector
```

This will read the `$SLURM_JOB_ID` environment variable.

When using the Docker container, the environment variable has to be passed explicitly:


```bash
# Divert stdout and sterr. Make sure the slurmuser has write access to both locations.
exec >> /epilog_logs/epilog.log
exec 2>> /epilog_logs/epilog.log

docker run -e SLURM_JOB_ID=$SLURM_JOB_ID -e AUDITOR_ADDR=auditor_host_address -e AUDITOR_PORT=8000 aluschumacher/auditor-slurm-epilog-collector:latest
```

TODO: This is likely not complete, because the container probably needs access to the host network. Test this.

### Filtering which records should be sent to Auditor

Filtering should be done in the epilog script. Only call the collector for jobs which should be sent to Auditor.

## License

TODO
