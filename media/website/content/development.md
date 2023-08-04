+++
title = "Development"
description = "Instructions for developers"
weight = 3
+++

This document covers instructions for developers of Auditor and its plugins/collectors.

# Compiling Auditor from source

This section describes how to set up the required development environment in order to compile Auditor from source.
It also covers cross compiling and static linking against `musl` instead of `glibc` in order to obtain a maximally portable binary.

## Prerequisites

Compiling Auditor requires

* Rust (see below)
* Docker
* sqlx-cli (see below)
* PostgreSQL client (`psql`)
* bunyan (optional, see below)


### Rust

Requires a recent Rust version (MSRV 1.56.1) and cargo.

Ideally installed via rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### sqlx

```bash
cargo install --version=0.6.3 sqlx-cli --no-default-features --features postgres,rustls
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
./scripts/init_db.sh
./scripts/init_slurm_collector_sqlite.sh
cargo run
```

Calling `./scripts/db_init.sh` will start a temporary PostgreSQL database in a Docker container and will automatically migrate the database.
If you plan to run Auditor like this in production, make sure to properly set up a database and instead of calling `db_init.sh` pass the correct settings to auditor via the configuration environment variables mentioned above.
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
The exact packages needed depends on your Linux distribution.
Furthermore, the `x86_64-unknown-linux-musl` target for Rust must be installed (once):

```bash
rustup target add x86_64-unknown-linux-musl
```

Then cross-compilation with static linking can be done via

```bash
RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl
```

The resulting binaries will be placed in `target/x86_64-unknown-linux-musl/release`.

# Github pages

This webpage is based on the [Zola](https://www.getzola.org/) framework.

For local development, [install](https://www.getzola.org/documentation/getting-started/installation/) the `zola` CLI program and run

```bash
 zola -r media/website serve
```
in the root directory of the Auditor repo.

The local version of the webpage is now available at [http://127.0.0.1:1111/](http://127.0.0.1:1111/).
