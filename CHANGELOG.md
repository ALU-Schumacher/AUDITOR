# Changelog
All notable changes to the AUDITOR project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking changes
- API: Deprecate get_[started/stopped]_since endpoint (#525) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Apel plugin: Remove `site_name_mapping` config parameter and change structure of `sites_to_report` config parameter ([@dirksammel](https://github.com/dirksammel))

### Security
- [RUSTSEC-2023-0071]: Ignored, because vulnerable code is not actually used by us ([@QuantumDancer](https://github.com/QuantumDancer)
- [RUSTSEC-2023-0074]: Update zerocopy from 0.7.26 to 0.7.31 ([@QuantumDancer](https://github.com/QuantumDancer))

### Added
- Auditor+pyauditor: Added advanced filtering when querying records (#466) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Auditor+pyauditor: Added bulk_insert option to insert list of records using auditor client and pyauditor (#580) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Auditor: Incorrect query string returns an error (#598) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Auditor: Return correct status code for errors during querying of records (#620) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Auditor: Remove get_since.rs and clean up dead code (#624) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- pyauditor: Add string representation to python Record Object (#596) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- CI: Added new workflow to define reusable parameters for other workflows ([@dirksammel](https://github.com/dirksammel))
- Docs: Add versioning of GitHub Pages and pyauditor docs (#551) ([@QuantumDancer](https://github.com/QuantumDancer)
- Docs: Add overview of API endpoints (#597) ([@QuantumDancer](https://github.com/QuantumDancer)
- Apel plugin: Add optional config setting for style of summary message ([@dirksammel](https://github.com/dirksammel))

### Changed
- Auditor+pyauditor: Deprecate `get_started_since()` and `get_stopped_since()` functions ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Auditor: Restructure `/record` endpoint to handle single record operations and `/records` endpoint to handle multiple records operations (#629) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Auditor: Incorrect meta and component query returns an empty vector and implement more edge case testing for advanced queries (#638) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Auditor: Implement prepared SQL queries using push_bind for advanced filtering (#637) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Dependencies: Update actions/setup-python from 4 to 5 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update actions/download-artifact from 3 to 4 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update actions/upload-artifact from 3 to 4 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update actix-web from 4.4.0 to 4.4.1 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update anyhow from 1.0.75 to 1.0.79 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update chrono from 0.4.31 to 0.4.33 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update cryptography from 41.0.5 to 42.0.0 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update fake from 2.9.1 to 2.9.2 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update h2 from 0.3.22 to 0.3.24 ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Dependencies: Update itertools from 0.12.0 to 0.12.1 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update once_cell from 1.18.0 to 1.19.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update opentelemetry_sdk from 0.21.1 to 0.21.2 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update pytest from 7.4.3 to 8.0.0 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update regex from 1.10.2 to 1.10.3 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update reqwest from 0.11.22 to 0.11.23 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update serde from 1.0.193 to 1.0.196 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update serde-aux from 4.2.0 to 4.4.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update serde_json from 1.0.108 to 1.0.113 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update serde_qs from 0.11.0 to 0.12.0 ([@QuantumDancer](https://github.com/QuantumDancer)
- Dependencies: Update serde_with from 3.4.0 to 3.5.1 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update setuptools from 69.0.2 to 69.0.3 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update sqlx from 0.7.2 to 0.7.3 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update thiserror from 1.0.50 to 1.0.56 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update tokio from 1.34.0 to 1.35.1 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update uuid from 1.6.1 to 1.7.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update wiremock from 0.5.21 to 0.5.22 ([@QuantumDancer](https://github.com/QuantumDancer))
- CI: Replace unmaintained actions-rs/audit-check action with maintained one from rustsec ([@QuantumDancer](https://github.com/QuantumDancer))
- CI: Introduce dependency between pyauditor release and release of python packages ([@dirksammel](https://github.com/dirksammel))
- Apel plugin: Replace all URL encodings in meta fields with single-character equivalent ([@dirksammel](https://github.com/dirksammel))
- Apel plugin: Use advanced querying for filtering records ([@dirksammel](https://github.com/dirksammel))
- Docs: Pyauditor- Fix pyauditor tutorial for creating new records (#631) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))

### Removed

## [0.3.1] - 2023-11-24

### Breaking changes

### Security
 
### Added
- Docs: Add steps for creating a new release ([@QuantumDancer](https://github.com/QuantumDancer))
- CI: Add Python 3.12 to workflows ([@dirksammel](https://github.com/dirksammel))

### Changed
- Dependencies: Update config from 0.13.3 to 0.13.4 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update pytest from 7.4.2 to 7.4.3 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update serde from 1.0.192 to 1.0.193 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update setuptools from 68.2.2 to 69.0.2 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update uuid from 1.5.0 to 1.6.1 ([@QuantumDancer](https://github.com/QuantumDancer))
- CI: Fix maturin version to v1.2.3 ([@QuantumDancer](https://github.com/QuantumDancer))

### Removed

## [0.3.0] - 2023-11-17

### Breaking changes
- Auditor: Standardize REST APIs. Routes have changed to single endpoint '/record' with methods such as 'GET', 'PUT', 'POST' (#465) ([@raghuvar-vijay](https://github.com/raghuvar-vijay)) 
- Priority plugin: 'auditor' configuration has to be present in the config file. 'prometheus' configuration is optional (#456) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Slurm collector: New filter options to filter slurm jobs are added. The `job_status` key in the config is moved to the `job_filter` section and is renamed to `status` (#472) ([@QuantumDancer](https://github.com/QuantumDancer))

### Security

### Added
- Auditor: Add records_handler module to routes to handle record queries  (#465) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Docs: Add instructions for developers for building the Rust and Python documentation locally ([@QuantumDancer](https://github.com/QuantumDancer))
- Priority plugin: Add prometheus data exporter (#456) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Slurm collector: Add `default_value` option for component configuration (#510) ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Add urlencoding 2.1.3 (to parse datetime while querying records)  (#465) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))


### Changed
- Slurm collector: Fix ambiguous local time in database.rs after switching from CEST to CET (#518) ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Slurm collector: Fix panic during component construction when job info is missing data for component (#510) ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update actix-web-opentelemetry 0.15.0 to 0.16.0 ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Dependencies: Update cargo-get from 0.3.3 to 1.0.0 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update cryptography from 41.0.4 to 41.0.5 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update fake from 2.8.0 to 2.9.1 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update itertools from 0.11.0 to 0.12.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update num-traits from 0.2.16 to 0.2.27 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update opentelemetry from 0.20.0 to 0.21.0 ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Dependencies: Update opentelemetry-prometheus from 0.13.0 to 0.14.1 ([@raghuvar-vijay](https://github.com/raghuvar-vijay), [@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update opentelemetry_sdk from 0.20.0 to 0.21.1 ([@raghuvar-vijay](https://github.com/raghuvar-vijay), [@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update regex from 1.9.5 to 1.10.2 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update reqwest from 0.11.20 to 0.11.22 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update rustix from 0.38.14 to 0.38.20 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update serde from 1.0.188 to 1.0.192 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update serde_json from 1.0.107 to 1.0.108 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update serde_with from 3.3.0 to 3.4.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update sqlx from 0.7.1 to 0.7.2 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update thiserror from 1.0.48 to 1.0.50 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update time from 0.3.28 to 0.3.29 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update tokio from 1.32.0 to 1.34.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update tracing from 0.1.37 to 0.1.40 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update tracing-actix-web from 0.7.6 to 0.7.9 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update tracing-log from 0.1.3 to 0.2.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update tracing-subscriber from 0.3.16 to 0.3.17 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update uuid from 1.4.1 to 1.5.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update wiremock from 0.5.19 to 0.5.21 ([@QuantumDancer](https://github.com/QuantumDancer))
- CI: Build pyauditor and Auditor Docker image from source for HTCondor collector test ([@dirksammel](https://github.com/dirksammel))
- CI: Build pyauditor with maturin for Python unit tests ([@dirksammel](https://github.com/dirksammel))
- CI: New workflow structure ([@dirksammel](https://github.com/dirksammel))

### Removed


## [0.2.0] - 2023-09-21

### Breaking changes
- pyauditor + Apel plugin + HTCondor collector: Support for Python 3.6 and 3.7 has been dropped ([@QuantumDancer](https://github.com/QuantumDancer))
- Apel plugin: `cpu_time_unit` has to be present in the config file. See [Documentation](https://github.com/ALU-Schumacher/AUDITOR/blob/main/media/website/content/_index.md#apel-plugin) ([@dirksammel](https://github.com/dirksammel))
- Auditor: Updating a non-existent record now returns an HTTP 404 error instead of HTTP 400 error ([@QuantumDancer](https://github.com/QuantumDancer))
- Docker containers: The `main` tag was replaced with the `edge` tag ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update pyo3 from 0.15.2 to 0.19 and pyo3-asyncio from 0.15 to 0.19 ([@QuantumDancer](https://github.com/QuantumDancer))
  - When creating a record with `pyauditor`, the timezone of the datetime object now needs to be converted to `datetime.timezone.utc` instead of `pytz.utc`

### Security
- [RUSTSEC-2023-0052]: Update webpki from 0.22.0 to 0.22.1 ([@dirksammel](https://github.com/dirksammel))
- [CVE-2022-35737]: Update libsqlite3-sys from 0.24.2 to 0.26.0 ([@dirksammel](https://github.com/dirksammel))

### Added
- Auditor + Apel plugin: Add semver tags and edge tag for docker container ([@QuantumDancer](https://github.com/QuantumDancer))
- Auditor + CI: Add integration test for Auditor Prometheus exporter([@QuantumDancer](https://github.com/QuantumDancer))
- Apel plugin: Migrate the Apel plugin from [ALU-Schumacher/AUDITOR-APEL-plugin](https://github.com/ALU-Schumacher/AUDITOR-APEL-plugin) to this repo ([@dirksammel](https://github.com/dirksammel))
- Apel plugin: Docker image ([@QuantumDancer](https://github.com/QuantumDancer))
- Apel plugin: Check if there are sites to report in the record list ([@dirksammel](https://github.com/dirksammel))
- HTCondor collector ([@rfvc](https://github.com/rfvc))
- Priority plugin: Add option for client timeout ([@QuantumDancer](https://github.com/QuantumDancer))
- Set LogLevel using env variable for auditor, slurm and slurm-epilog collectors and priority plugin ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- CI: Linting of python code with ruff and black ([@dirksammel](https://github.com/dirksammel))
- CI: Dependabot checks for python plugins/collectors and GitHub actions ([@dirksammel](https://github.com/dirksammel))
- CI: Python unit tests ([@dirksammel](https://github.com/dirksammel))
- CI: Check CHANGELOG.md for changes ([@dirksammel](https://github.com/dirksammel))
- CI: Publish Apel plugin and HTCondor collector to PyPI and GitHub ([@dirksammel](https://github.com/dirksammel))
- Docs: Document `Record`, `RecordAdd`, `RecordUpdate`, `Component`, `Score`, and `Meta` in Rust API ([@QuantumDancer](https://github.com/QuantumDancer))
- Docs: Add tutorial for Rust client ([@QuantumDancer](https://github.com/QuantumDancer))
- Docs: Common sections for collectors and plugins ([@QuantumDancer](https://github.com/QuantumDancer))
- Docs: Dedicated docs for development ([@QuantumDancer](https://github.com/QuantumDancer))
- Docs: Add documentation for the Slurm collector ([@QuantumDancer](https://github.com/QuantumDancer))
- Docs: Add documentation for the Apel plugin ([@dirksammel](https://github.com/dirksammel))

### Changed
- Apel plugin: Bugfix in catching empty record list ([@dirksammel](https://github.com/dirksammel))
- Apel plugin: Catch VOMS information that does not start with `/` ([@dirksammel](https://github.com/dirksammel))
- Apel plugin: Remove `pytz` dependency ([@dirksammel](https://github.com/dirksammel))
- Apel plugin: Refactor code ([@dirksammel](https://github.com/dirksammel))
- Apel plugin: Remove encoding from logging ([@dirksammel](https://github.com/dirksammel))
- Auditor: Fix default address in AuditorClientBuilder ([@QuantumDancer](https://github.com/QuantumDancer))
- CI: Update list of RUSTSEC ignores ([@dirksammel](https://github.com/dirksammel))
- HTCondor collector: Handle `undefined` values from the batch system correctly ([@rfvc](https://github.com/rfvc))
- HTCondor collector: Replace `datetime.utcfromtimestamp` with `datetime.fromtimestamp` ([@dirksammel](https://github.com/dirksammel))
- Slurm collector: Add option to allow for empty fields in `sacct` output ([@QuantumDancer](https://github.com/QuantumDancer))
- Slurm collector: Fix parsing of ParsableType::Time for certain cases ([@QuantumDancer](https://github.com/QuantumDancer))
- Webpage: Adjust color schema and add text to About and ChangeLog pages ([@frboehler](https://github.com/frboehler), [@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update actions/checkout from 1 to 4 ([@dirksammel](https://github.com/dirksammel), [@rfvc](https://github.com/rfvc))
- Dependencies: Update actions/download-artifact from 2 to 3 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update actions/setup-python from 2 to 4 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update actions/upload-artifact from 2 to 3 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update actix-web from 4.3.1 to 4.4.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update actix-web-opentelemetry from 0.12.0 to 0.15.0 ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Dependencies: Update anyhow from 1.0.70 to 1.0.75 ([@stefan-k](https://github.com/stefan-k), [@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update chrono from 0.4.24 to 0.4.31 ([@dirksammel](https://github.com/dirksammel), [@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update cryptography from 41.0.1 to 41.0.4 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update docker/build-push-action from 2.5.0 to 5 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update docker/login-action from 1.10.0 to 3 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update docker/metadata-action from 3.3.0 to 5 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update docker/setup-buildx-action from 1 to 3 ([@dirksammel](https://github.com/dirksammel))
- Dependencies: Update fake from 2.5.0 to 2.8.0 ([@stefan-k](https://github.com/stefan-k), [@QuantumDancer](https://github.com/QuantumDancer), [@dirksammel](https://github.com/dirksammel))
- Dependencies: Update itertools from 0.10.5 to 0.11.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update maturin from 0.13 to 1.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update num-traits from 0.2.15 to 0.2.16 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update once_cell from 1.17.1 to 1.18.0 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update opentelemetry from 0.17.0 to 0.20.0 ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Dependencies: Update opentelemetry-prometheus from 0.10.0 to 0.13.0 ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Dependencies: Update regex from 1.7.3 to 1.9.5 ([@stefan-k](https://github.com/stefan-k), [@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update reqwest from 0.11.16 to 0.11.20 ([@stefan-k](https://github.com/stefan-k), [@QuantumDancer](https://github.com/QuantumDancer), [@dirksammel](https://github.com/dirksammel))
- Dependencies: Update rustls-webpki from 0.101.2 to 0.101.4 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update serde from 1.0.160 to 1.0.188 ([@stefan-k](https://github.com/stefan-k), [@QuantumDancer](https://github.com/QuantumDancer), [@dirksammel](https://github.com/dirksammel))
- Dependencies: Update serde_json from 1.0.96 to 1.0.107 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update serde_with from 2.3.2 to 3.3.0 ([@stefan-k](https://github.com/stefan-k), [@QuantumDancer](https://github.com/QuantumDancer), [@dirksammel](https://github.com/dirksammel))
- Dependencies: Update sqlx from 0.6.3 to 0.7.1 ([@dirksammel](https://github.com/dirksammel)), ([@raghuvar-vijay](https://github.com/raghuvar-vijay))
- Dependencies: Update thiserror from 1.0.40 to 1.0.48 ([@QuantumDancer](https://github.com/QuantumDancer), [@dirksammel](https://github.com/dirksammel))
- Dependencies: Update tokio from 1.27.0 to 1.32.0 ([@stefan-k](https://github.com/stefan-k), [@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update tracing from 0.1.37 to 0.1.38 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Update tracing-actix-web from 0.7.4 to 0.7.6 ([@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update tracing-bunyan-formatter from 0.3.7 to 0.3.9 ([@QuantumDancer](https://github.com/QuantumDancer), [@dirksammel](https://github.com/dirksammel))
- Dependencies: Update tracing-subscriber from 0.3.16 to 0.3.17 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Update uuid from 1.3.1 to 1.4.1 ([@stefan-k](https://github.com/stefan-k), [@QuantumDancer](https://github.com/QuantumDancer))
- Dependencies: Update wiremock from 0.5.18 to 0.5.19 ([@QuantumDancer](https://github.com/QuantumDancer))

### Removed

## [0.1.0] - 2023-04-19
### Added
- pyauditor: Added `to_json` method to records ([@stefan-k](https://github.com/stefan-k))
- pyauditor: Runtime can now be set for record either directly or by adding a `stop_time` ([@stefan-k](https://github.com/stefan-k))
- Slurm collector: Added JSON parsing for meta fields ([@stefan-k](https://github.com/stefan-k))
- Slurm collector: Added VOMS proxy info to meta ([@stefan-k](https://github.com/stefan-k))
- Auditor + pyauditor: Added blocking client ([@stefan-k](https://github.com/stefan-k))
- Auditor: Added implementation for `From` trait to `ClientError` ([@QuantumDancer](https://github.com/QuantumDancer))

### Changed
- Slurm collector: Bugfixes for parsing slurm output ([@stefan-k](https://github.com/stefan-k))
- Slurm collector: Ignores subjobs ([@stefan-k](https://github.com/stefan-k))
- Slurm collector: Use safer defaults ([@rkleinem](https://github.com/rkleinem))
- Dependencies: Updated actix-web from 4.3.0 to 4.3.1 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated anyhow from 1.0.69 to 1.0.70 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated chrono from 0.4.23 to 0.4.24 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated once_cell from 1.17.0 to 1.17.1 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated regex from 1.7.1 to 1.7.3 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated reqwest from 0.11.14 to 0.11.16 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated serde from 1.0.152 to 1.0.160 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated serde-aux from 4.1.2 to 4.2.0 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated serde_json from 1.0.93 to 1.0.96 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated serde_with from 2.2.0 to 2.3.2 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated sqlx from 0.6.2 to 0.6.3 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated thiserror from 1.0.38 to 1.0.40 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated tokio from 1.25.0 to 1.27.0 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated tracing-actix-web from 0.7.2 to 0.7.4 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated tracing-bunyan-formatter from 0.3.6 to 0.3.7 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated uuid from 1.3.0 to 1.3.1 ([@stefan-k](https://github.com/stefan-k))
- Dependencies: Updated wiremock from 0.5.17 to 0.5.18 ([@stefan-k](https://github.com/stefan-k))

### Removed
- Auditor: Removed constructors for auditor client (both async and blocking) ([@QuantumDancer](https://github.com/QuantumDancer))

## [0.0.7] - 2023-02-13
### Added
- Added Slurm collector ([@stefan-k](https://github.com/stefan-k))
- Added code coverage to CI ([@stefan-k](https://github.com/stefan-k))

### Changed
- All collectors and plugins are now dedicated crates ([@stefan-k](https://github.com/stefan-k))
- Renamed Score "factor" to "value" ([@stefan-k](https://github.com/stefan-k))
- Added meta field which stores key-value pairs of the form `String -> Vec<string>` ([@stefan-k](https://github.com/stefan-k))
- Auditor crate now has server and client features. This allows one to avoid pulling in server code when only client code is needed. Server code requires a live database to compile (because of sqlx). ([@stefan-k](https://github.com/stefan-k))
- Support for building python 3.11 pyauditor modules ([@stefan-k](https://github.com/stefan-k))
- Improvements in CI ([@stefan-k](https://github.com/stefan-k))
- Replaced `cargo-spellcheck` with `typos` ([@stefan-k](https://github.com/stefan-k))
- Updated Postgres instances in CI to version 15 ([@stefan-k](https://github.com/stefan-k))
- Use claims instead of unmaintained claim ([@stefan-k](https://github.com/stefan-k))
- Removed dependency on time 0.1 as much as possible. Potential vulnerability does not affect us though. ([@stefan-k](https://github.com/stefan-k))
- Updated tokio from 1.22.0 to 1.25.0 ([@stefan-k](https://github.com/stefan-k))
- Updated prometheus from 0.13.1 to 0.13.3 ([@stefan-k](https://github.com/stefan-k))
- Updated serde_with from 2.0.0 to 2.2.0 ([@stefan-k](https://github.com/stefan-k))
- Updated actix-web from 4.1.0 to 4.3.0 ([@stefan-k](https://github.com/stefan-k))
- Updated anyhow from 1.0.64 to 1.0.69 ([@stefan-k](https://github.com/stefan-k))
- Updated thiserror from 1.0.34 to 1.0.37 ([@stefan-k](https://github.com/stefan-k))
- Updated unicode-segmentation from 1.9.0 to 1.10.1 ([@stefan-k](https://github.com/stefan-k))
- Updated reqwest from 0.11.11 to 0.11.14 ([@stefan-k](https://github.com/stefan-k))
- Updated tracing-actix-web from 0.6.0 to 0.7.1 ([@stefan-k](https://github.com/stefan-k))
- Updated once_cell from 1.14.0 to 1.17.0 ([@stefan-k](https://github.com/stefan-k))
- Updated sqlx from 0.6.1 to 0.6.2 ([@stefan-k](https://github.com/stefan-k))
- Updated serde from 1.0.144 to 1.0.152 ([@stefan-k](https://github.com/stefan-k))
- Updated tracing-subscriber from 0.3.15 to 0.3.16 ([@stefan-k](https://github.com/stefan-k))
- Updated tracing-bunyan-formatter from 0.3.3 to 0.3.6 ([@stefan-k](https://github.com/stefan-k))
- Updated uuid from 1.1.2 to 1.3.0 ([@stefan-k](https://github.com/stefan-k))
- Updated wiremock from 0.5.14 to 0.5.17 ([@stefan-k](https://github.com/stefan-k))
- Updated config from 0.13.2 to 0.13.3 ([@stefan-k](https://github.com/stefan-k))
- Updated regex from 1.7.0 to 1.7.1 ([@stefan-k](https://github.com/stefan-k))

### Removed
- Removed old python client ([@stefan-k](https://github.com/stefan-k))
- Removed `user_id`, `site_id` and `group_id` from `Record` ([@stefan-k](https://github.com/stefan-k))

## [0.0.6] - 2022-09-06
### Added
- Spellcheck in CI ([@stefan-k](https://github.com/stefan-k)).
- cargo-deny in CI ([@stefan-k](https://github.com/stefan-k)).
- Implemented comparison operators for pyauditor types ([@stefan-k](https://github.com/stefan-k)).

### Changed
- Any `get` endpoint now returns a list of records sorted by `stop_time` ([@stefan-k](https://github.com/stefan-k)).
- Updated anyhow from 1.0.63 to 1.0.64 ([@stefan-k](https://github.com/stefan-k)).
- Updated thiserror from 1.0.33 to 1.0.34 ([@stefan-k](https://github.com/stefan-k)).
- Updated serde-aux from 3.1.0 to 4.0.0 ([@stefan-k](https://github.com/stefan-k)).
- Updated once-cell from 1.13.1 to 1.14.0 ([@stefan-k](https://github.com/stefan-k)).
- Updated sqlx from 0.5.7 to 0.6.1 ([@stefan-k](https://github.com/stefan-k)).

### Fixed
- Fixed Slurm Epilog Collector to correctly send UTC timestamps ([@stefan-k](https://github.com/stefan-k)).

### Deprecated
- Old python client written in python is deprecated ([@stefan-k](https://github.com/stefan-k)).

## [0.0.5] - 2022-08-25
### Added
- Database metrics in Prometheus exporter ([@stefan-k](https://github.com/stefan-k)).
- Added cargo-deny to CI ([@stefan-k](https://github.com/stefan-k)).

### Changed
- Better errors, error handling, error logging and exposing errors to users ([@stefan-k](https://github.com/stefan-k)).
- Using a SQL transaction for updating records ([@stefan-k](https://github.com/stefan-k)).
- pyauditor wheels now also have support for python 3.6 (for TARDIS). This required downgrading the pyo3 libraries ([@stefan-k](https://github.com/stefan-k)).
- Restructured and simplified test suite ([@stefan-k](https://github.com/stefan-k)).
- AuditorClient now properly errors on server errors ([@stefan-k](https://github.com/stefan-k)).
- Updated once-cell from 1.13.0 to 1.13.1 ([@stefan-k](https://github.com/stefan-k)).
- Updated anyhow from 1.0.61 to 1.0.62 ([@stefan-k](https://github.com/stefan-k)).
- Updated serde from 1.0.143 to 1.0.144 ([@stefan-k](https://github.com/stefan-k)).

### Fixed
- Fixed broken website build in CI ([@stefan-k](https://github.com/stefan-k)).
- Removed duplicate configuration directory ([@stefan-k](https://github.com/stefan-k)).

## [0.0.4] - 2022-08-16
### Added
- Sphinx documentation for pyauditor module ([@stefan-k](https://github.com/stefan-k)).
- Tutorial for pyauditor module ([@stefan-k](https://github.com/stefan-k)).
- Automatic deployment of pyauditor documentation ([@stefan-k](https://github.com/stefan-k)).

### Changed
- Updated chrono from 0.4.21 to 0.4.22 ([@stefan-k](https://github.com/stefan-k)).

### Fixed
- Correct badges for pyauditor Readme ([@stefan-k](https://github.com/stefan-k)).
- Moved sqlx-data.json to auditor folder to fix docs.rs build ([@stefan-k](https://github.com/stefan-k)).

## [0.0.3] - 2022-08-11
### Added
- Python interface exported from Rust code (pyauditor) including test harness ([@stefan-k](https://github.com/stefan-k)).
- Logging spans with unique id for priority plugin and slurm epilog collector (helps differentiate different runs in logs) ([@stefan-k](https://github.com/stefan-k)).
- Export of HTTP metrics on `/metrics` endpoint for prometheus (Auditor) ([@stefan-k](https://github.com/stefan-k)).
- Builder pattern for `AuditorClient` (`AuditorClientBuilder`) ([@stefan-k](https://github.com/stefan-k)).
- Unit tests for client code ([@stefan-k](https://github.com/stefan-k)).
- Build pipeline for python wheels on Linux, Windows and MacOS for python versions 3.7-3.10 ([@stefan-k](https://github.com/stefan-k)).
- Added python package description ([@stefan-k](https://github.com/stefan-k)).
- Added pyauditor readme ([@stefan-k](https://github.com/stefan-k)).

### Changed
- `add` and `update` methods of `AuditorClient` now take references to `Record` ([@stefan-k](https://github.com/stefan-k)).
- Updated config from 0.13.1 to 0.13.2 ([@stefan-k](https://github.com/stefan-k)).
- Updated serde from 1.0.141 to 1.0.143 ([@stefan-k](https://github.com/stefan-k)).
- Updated chrono from 0.4.19 to 0.4.21 ([@stefan-k](https://github.com/stefan-k)).
- Updated wiremock from 0.5.13 to 0.5.14 ([@stefan-k](https://github.com/stefan-k)).
- Updated anyhow from 1.0.60 to 1.0.61 ([@stefan-k](https://github.com/stefan-k)).
- Introduced workspaces (as preparation for the python client written in Rust) ([@stefan-k](https://github.com/stefan-k)).
- Better error handling in Auditor client code ([@stefan-k](https://github.com/stefan-k)).
- Improved API of `Component` type ([@stefan-k](https://github.com/stefan-k)).
- CI: Moved clippy pipeline to beta channel ([@stefan-k](https://github.com/stefan-k)).
- Changed some of the interfaces in `domain` module to better fit pyauditor ([@stefan-k](https://github.com/stefan-k)).

### Fixed
- Pointed auditor Cargo.toml to correct readme ([@stefan-k](https://github.com/stefan-k)).

## [0.0.2] - 2022-08-01
### Added
- Documentation of priority plugin on website ([@stefan-k](https://github.com/stefan-k)).

### Changed
- CI: Run clippy for all targets ([@stefan-k](https://github.com/stefan-k)).
- Build docker containers when pushing a version tag ([@stefan-k](https://github.com/stefan-k)).
- Updated tracing from 1.0.35 to 1.0.36 ([@stefan-k](https://github.com/stefan-k)).

### Fixed
- Correctly parse scontrol output in slurm epilog collector (Thanks to Raphael Kleinemuehl for the hint!) ([@stefan-k](https://github.com/stefan-k)).
- Fixed building of docs on docs.rs by activating sqlx offline mode ([@stefan-k](https://github.com/stefan-k)).

## [0.0.1] - 2022-07-26
### Added
- Auditor ([@stefan-k](https://github.com/stefan-k)).
- Auditor slurm epilog collector ([@stefan-k](https://github.com/stefan-k)).
- Auditor priority plugin ([@stefan-k](https://github.com/stefan-k)).
- Auditor website ([@stefan-k](https://github.com/stefan-k)).
- Docker container builds ([@stefan-k](https://github.com/stefan-k)).
- RPM builds ([@stefan-k](https://github.com/stefan-k)).



[Unreleased]: https://github.com/alu-schumacher/AUDITOR/compare/v0.3.1...HEAD
[0.0.1]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.1
[0.0.2]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.2
[0.0.3]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.3
[0.0.4]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.4
[0.0.5]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.5
[0.0.6]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.6
[0.0.7]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.7
[0.1.0]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.1.0
[0.2.0]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.2.0
[0.3.0]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.3.0
[0.3.1]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.3.1
