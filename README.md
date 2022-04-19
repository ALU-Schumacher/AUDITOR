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


## License

TODO
