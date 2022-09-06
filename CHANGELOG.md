# Changelog
All notable changes to the AUDITOR project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- CI: Moved clippy pipline to beta channel ([@stefan-k](https://github.com/stefan-k)).
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



[Unreleased]: https://github.com/alu-schumacher/AUDITOR/compare/v0.0.6...HEAD
[0.0.1]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.1
[0.0.2]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.2
[0.0.3]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.3
[0.0.4]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.4
[0.0.5]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.5
[0.0.6]: https://github.com/alu-schumacher/AUDITOR/releases/tag/v0.0.6
