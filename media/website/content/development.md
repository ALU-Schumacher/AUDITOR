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
cargo install --version=0.7.3 sqlx-cli --no-default-features --features postgres,rustls
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

# Adding new plugins and collectors

TODO: Instructions for adding new plugins/collectors (especially directory structure, CI config, etc)

# Documentation

## Github pages

This webpage is based on the [Zola](https://www.getzola.org/) framework.

For local development, [install](https://www.getzola.org/documentation/getting-started/installation/) the `zola` CLI program and run

```bash
 zola -r media/website serve
```
in the root directory of the Auditor repo.

The local version of the webpage is now available at [http://127.0.0.1:1111/](http://127.0.0.1:1111/).

## Rust documentation

A local version of the Rust documentation can be built with

```bash
cargo doc
```

Use the`--open` flag to directly open the documentation in your browser.

## Python client

The documentation of the python client is based on the [Sphinx](https://www.sphinx-doc.org/) framework.
A local version of the documentation can be built with

```bash
scripts/docs_pyauditor.sh
```

The documentation can then be found in `pyauditor/docs/_build/html/`.

# Creating a new release

Follow the steps below in order to create a new release.

Example PR: [https://github.com/ALU-Schumacher/AUDITOR/pull/547](https://github.com/ALU-Schumacher/AUDITOR/pull/547)

- Update the version number in all `Cargo.toml` files
- Run `cargo update` to update dependencies in `Cargo.lock`
- Update the version number in all `pyproject.toml` files
- Finalize the [changelog](https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md)
  - Rename `Unreleased` to version number, add date
  - Add new `Unreleased` section with all subsections (Breaking changes, Security, Added, Changed, Removed)
  - At the bottom: Add link target for new version
  - At the bottom: Update link target for unreleased version
- Finalize the [migration guide](https://github.com/ALU-Schumacher/AUDITOR/blob/main/media/website/content/migration.md)
  - Rename `Unreleased` to version number
- Update changelog in [RPM `.spec` files](https://github.com/ALU-Schumacher/AUDITOR/tree/main/rpm)
- Create PR and wait for approval from other team member
- Publish `auditor` crate first (you will need a [crates.io API token](https://crates.io/settings/tokens))
  ```bash
  cd auditor
  cargo publish --dry-run
  cargo publish
  ```
- Then run the publish workflow for all rust-based collectors/plugins (`cd` into corresponding dirs)
  - `plugins/priority`
  -  `collectors/slurm` (prepend `cargo` commands with `SQLX_OFFLINE=true`)
  - `collectors/slurm_epilog`
- Merge PR
- Create tag for version
  ```bash
  git fetch upstream
  git checkout upstream/main
  git tag <version>  # e.g. v0.1.0
  git push upstream <version>
  ```
  - This triggers the build of the pyauditor package and the python-based collectors/plugins
  - This triggers the build of the docker containers and pushes them to DockerHub and GHCR
  - This triggers a GitHub release
    - Update the release notes by editing the release and pressing the auto-generate button
- Announce in Mattermost AUDITOR channel
- Update the `pyauditor` version number in [tardis](https://github.com/MatterMiners/tardis) (and update the code of the AUDITOR plugin if the release introduced breaking changes)
