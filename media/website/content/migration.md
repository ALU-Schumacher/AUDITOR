+++
title = "Migration"
description = "Migration Guide"
weight = 3
+++

# From 0.8.0 to unreleased

# From 0.7.1 to 0.8.0

## Apel plugin

- Due to the switch to the ARGO AMS library, the config file has to be adjusted. The `authentication` section needs to be replaced with the new `messaging` section. Please have a look at the example config in the [documentation](https://alu-schumacher.github.io/AUDITOR/latest/#apel-plugin).
- The `NormalisedWallDurationField` class was removed, but the default `base_value` of the `NormalisedField` class is now the `runtime` of the record. Replace `NormalisedWallDuration: !NormalisedWallDurationField` with `NormalisedWallDuration: !NormalisedField` in the config file.

# From 0.7.0 to 0.7.1

Please backup your db before proceeding with any changes that are listed below.

## Remove forbidden characters:
The following changes only apply to users who are either using HTCondor collector (v0.6.3 and earlier) or slurm collector (v0.6.3 and earlier), follow these steps to revert the encodings in your database records:

### HTCondor collector v0.6.3 and earlier or Slurm collector v0.6.3 and earlier
- Clone the github repository [git_repo](https://github.com/ALU-Schumacher/AUDITOR).
- Move into the cloned repository `cd AUDITOR`
- Install the required dependencies with `pip install -r auditor/scripts/revert_encoding/requirements.txt`
- Replace the placeholder values in the .env file present at `auditor/scripts/revert_encoding` with the values corresponding to your database config.
- Run `python auditor/scripts/revert_encoding/revert_encodings.py`

# From 0.6.3 to 0.7.0

### Update to [sqlx 0.8.3](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md)
Use this command to update the sqlx-cli to 0.8.3
- `cargo install --version=0.8.3 sqlx-cli --no-default-features --features postgres,rustls,sqlite`

Please backup your db before proceeding with any changes that are listed below.

## Remove forbidden characters:
The following changes only apply to users who are either using HTCondor collector (v0.6.3 and earlier) or slurm collector (v0.6.3 and earlier), follow these steps to revert the encodings in your database records:

### HTCondor collector v0.6.3 and earlier or Slurm collector v0.6.3 and earlier
- Clone the github repository [git_repo](https://github.com/ALU-Schumacher/AUDITOR).
- The script for reverting encodings is located at: `auditor/scripts/revert_encoding`.
- Install the required dependencies with `pip install -r requirements.txt`
- Replace the placeholder values in the .env file with the values corresponding to your database config.
- Run `python revert_encodings.py`

### New feature - TLS

TLS is added to AUDITOR, all collectors and plugins. A new config section called tls_config is required by all config files. `use_tls` is a compulsory field of type bool that defines whether to use the TLS or not.

## TLS Configuration Table for AUDITOR (AUDITOR is referred to as the server)

| Parameter           | Type    | Description                                                                                    | Example Value                  | Required if `use_tls` is `true`  |
|---------------------|---------|------------------------------------------------------------------------------------------------|--------------------------------|----------------------------------|
| `use_tls`           | Boolean | Specifies whether TLS is enabled (`true`) or disabled (`false`).                               | `true` or `false`              | Yes                              |
| `ca_cert_path`      | String  | Path to the root Certificate Authority (CA) certificate for validating client certificates.    | `/path/rootCA.pem`             | Yes                              |
| `server_cert_path`  | String  | Path to the server's TLS certificate.                                                          | `/path/server-cert.pem`        | Yes                              |
| `server_key_path`   | String  | Path to the server's private key used for TLS.                                                 | `/path/server-key.pem`         | Yes                              |
| `https_addr`        | String  | The HTTPS address where the server will listen. Defaults to a pre-configured value if not set. | `"0.0.0.0"`                    | No                               |
| `https_port`        | Integer | The HTTPS port where the server will listen. Defaults to a pre-configured value if not set.    | `8003`                         | No                               |

An example config is provided at the following directory in the github repository `auditor/configuration/tls_config.yaml`

---

## TLS Configuration Table for Collectors and Plugins (All collectors and plugins are referred to as the clients)

| Parameter            | Type            | Description                                                                                 | Example Value                  | Required if `use_tls` is `true`  |
|----------------------|-----------------|---------------------------------------------------------------------------------------------|--------------------------------|----------------------------------|
| `use_tls`            | Boolean         | Specifies whether TLS is enabled (`true`) or disabled (`false`).                            | `true` or `false`              | Yes                              |
| `ca_cert_path`       | String          | Path to the root Certificate Authority (CA) certificate for validating server certificates. | `/path/rootCA.pem`             | Yes                              |
| `client_cert_path`   | String          | Path to the client's TLS certificate.                                                       | `/path/client-cert.pem`        | Yes                              |
| `client_key_path`    | String          | Path to the client's private key used for TLS.                                              | `/path/client-key.pem`         | Yes                              |
Please have a look at the AUDITOR documentation for the new changes in the config files for [collectors](https://alu-schumacher.github.io/AUDITOR/latest/#collectors) and [plugins](https://alu-schumacher.github.io/AUDITOR/latest/#plugins).

# From 0.6.2 to 0.6.3

### Update to [sqlx 0.8.2](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md)
Use this command to update the sqlx-cli to 0.8.2
- `cargo install --version=0.8.2 sqlx-cli --no-default-features --features postgres,rustls,sqlite`

# From 0.5.0 to 0.6.2

## Auditor DB:
- WARNING: Please create a backup of the database before running the migration script.
- AUDITOR db should be migrated to use the new schema. Run `sqlx migrate run --source migration` from the AUDITOR home directory. It is also possible using the container which can be found here [Documentation](../#migrating-the-database).

## Apel plugin

- The APEL message can now be configured via the config. An updated example config file can be found in the [Documentation](../#apel-plugin).
- `auditor-apel-republish` now needs the arguments `--begin-date` and `--end-date` instead of `--month` and `--year`. The format is `yyyy-mm-dd hh:mm:ss+00:00`, e.g. `2023-11-29 21:10:54+00:00`.

# From 0.4.0 to 0.5.0

## Apel plugin

- The format of the config file was changed from INI to YAML. An updated example config file can be found in the [Documentation](../#apel-plugin).
- The stop times of the latest reported job per site and the time of the latest report to APEL are now stored in a JSON file instead of a SQLite database. Therefore, the config parameter `time_db_path` has to be changed to `time_json_path`.
To migrate an existing database to a JSON file, run `migration-0_4_0-to-0_5_0.py` located in the `scripts` folder:

```bash
usage: migration-0_4_0-to-0_5_0.py [-h] -c CONFIG -d DB -j JSON

options:
  -h, --help            show this help message and exit
  -c CONFIG, --config CONFIG
                        Path to the config file
  -d DB, --db DB        Path to the time database file
  -j JSON, --json JSON  Path to the time JSON file
```

This already requires a config file with YAML format.

## Docker container

The Auditor Docker container can now be used to run database migrations. For details, see the [documentation](../#migrating-the-database).

If you run Auditor using the Docker container and provide an external config file, you need to change the way how you run the Docker container:

- Old
  ```bash
  docker run -v <absolute-path-to-config>:/auditor/config.yaml aluschumacher/auditor:<version> /auditor/config.yaml
  ```
- New
  ```bash
  docker run -v <absolute-path-to-config>:/auditor/config.yaml aluschumacher/auditor:<version> auditor /auditor/config.yaml
  ```

I.e., you need to add `auditor` as the first argument before pointing to the configuration file.

## Development

### Update to [sqlx 0.7.4](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md)
Use this command to update the sqlx-cli to 0.7.4
- `cargo install --version=0.7.4 sqlx-cli --no-default-features --features postgres,rustls,sqlite`

# From 0.3.0/0.3.1 to 0.4.0

## AUDITOR
### REST APIs

Auditor REST APIs are changed as shown in the table below. 

| Action              | Before                                      | After                                                 |
| ------------------- | ------------------------------------------- | ----------------------------------------------------- |
| Health check        | `/health_check` (GET)                       | `/health_check` (GET)                                 |
| Add record          | `/add` (POST)                               | `/record` (POST)                                      |
| Update record       | `/update` (POST)                            | `/record` (PUT)                                       |
| Insert Bulk records | Did not exist                               | `/records` (PUT)                                      |
| Get all records     | `/get` (GET)                                | `/records` (GET)                                      |
| Get records since   | `/get/[started/stopped]/since/{date}` (GET) | `/records?state=[started/stopped]&since={date}` (GET) |

`/record` endpoint handles single record operations such as adding one record, updating one record and querying one record.

`/records` endpoint handles multiple and bulk record operations such as inserting bulk records and querying multiple records.

## Apel plugin

The config parameter `site_name_mapping` is removed and the structure of the config parameter `sites_to_report` is changed. `sites_to_report` is now a dictionary, where the keys are the site names as configured in the GOCDB, and the values are lists of the corresponding site names in the AUDITOR records.

Before:
```python
sites_to_report = ["site_id_1", "site_id_2", "site_id_3"]
site_name_mapping = {"site_id_1": "SITE_A", "site_id_2": "SITE_A", "site_id_3": "SITE_B"}
```

After:
```python
sites_to_report = {"SITE_A": ["site_id_1", "site_id_2"], "SITE_B": ["site_id_3"]}
```

## Removed
`/get_[started/stopped]_since` endpoint is removed due to the introduction of advanced query. The auditor client and pyauditor client still contains the get_started_since and get_stopped_since
functions but throws a deprecated warning if used. 

## Development

### Update to [sqlx 0.7.3](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md)
Use this command to update the sqlx-cli to 0.7.3
- `cargo install --version=0.7.3 sqlx-cli --no-default-features --features postgres,rustls,sqlite`

# From 0.2.0 to 0.3.0

## Slurm collector

New filter options for querying slurm jobs are available.
Due to this, a new section `job_filter` has been introduced for the config file.
The `job_status` field has been renamed to `status` and is now part of the `job_filter` section.

Before:
```yaml
job_status:
  - "completed"
  - "failed"
```

After:
```yaml
job_filter:
  status:
    - "completed"
    - "failed"
```

The new filter options are `partition`, `user`, `group`, and `account` and work similar to the `status` filter.

## Priority plugin

The priority plugin now supports exporting metrics for the amount of provided resources and the updated priority to Prometheus.
The metrics can be accessed via a GET request to the `/metrics` endpoint.
Because the metrics endpoint provided by the Prometheus exporter needs to be available all the time, the architecture of the
priority plugin has been changed. It now will run continuously. In most cases, it should be started as a systemd service.

The structure of the config file has changed. The `addr` and `port` options are now put under a common `auditor` section.
The frequency of recalculation for the provided resources and priorities can now be controlled with the `frequency` field, which assumes that the number given is in seconds.
It defaults to 1 hour (i.e. 3600s).

The Prometheus exporter can be configured in the `prometheus` section.
It can be enabled and disabled with the `enable` field.
The address and port of the HTTP server that serves the metrics can be set with the `addr` and `port` fields.
The `metrics` list specifies the metrics that are exported. Right now the values `ResourceUsage` (for the amount of provided resources in the given duration)
and `Priority` (for the calculated priority value) are supported.
The `prometheus` section is optional. If it is not present, it has the same effect as setting `enable` to `false`.

Below, you find an example of the priority plugin configuration before and after the change.

- Before 
	```yaml
	addr: "localhost"
	port: 8000
	... (other options)
	```

 - After
	 ```yaml
	auditor:
	   addr: "localhost"
	   port: 8000
	frequency: 3600
	... (other options)
	prometheus:
	   enable: true
	   addr: "0.0.0.0"
	   port: 9000
	   metrics:
	     - ResourceUsage
	     - Priority
	 ```


## AUDITOR
### Standardized REST APIs

Auditor REST APIs are changed as shown in the table below. 

| Action            | Before                                      | After                                                |
| ----------------- | ------------------------------------------- | ---------------------------------------------------- |
| Health check      | `/health_check` (GET)                       | `/health_check` (GET)                                |
| Add record        | `/add` (POST)                               | `/record` (POST)                                     |
| Update record     | `/update` (POST)                            | `/record` (PUT)                                      |
| Get all records   | `/get` (GET)                                | `/record` (GET)                                      |
| Get records since | `/get/[started/stopped]/since/{date}` (GET) | `/record?state=[started/stopped]&since={date}` (GET) |


## Development

### Update to [sqlx 0.7.2](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md)
Use this command to update the sqlx-cli to 0.7.2
- `cargo install --version=0.7.2 sqlx-cli --no-default-features --features postgres,rustls,sqlite`

# From 0.1.0 to 0.2.0

## Apel plugin

- The config file now needs to have a field `cpu_time_unit` present, which describes the unit of total CPU time in the AUDITOR records.
  Possible values are `seconds` or `milliseconds`.
- Support for Python 3.6 and Python 3.7 has been dropped. Please move to a newer version of python.


## Auditor client

- Support for Python 3.6 and Python 3.7 has been dropped. Please move to a newer version of python.
- Due to the update of the `pyo3` library, the timezone of datetime objects now needs to be `datetime.timezone.utc` instead of `pytz.utc`
  when creating a new record:
  - When the datetime object is already in UTC
    - Before
      ```python
      import datetime
      import pytz

      start_since = datetime.datetime(2022, 8, 8, 11, 30, 0, 0, tzinfo=pytz.utc)
      ```
    - After
      ```python
      import datetime

      start_since = datetime.datetime(2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc)
      ```
  - If it is in local time
    - Before
      ```python
      import datetime
      import pytz
      from tzlocal import get_localzone

      local_tz = get_localzone()
      start_since = datetime.datetime(2022, 8, 8, 11, 30, 0, 0, tzinfo=local_tz).astimezone(pytz.utc)
      ```
    - After
      ```python
      import datetime
      from tzlocal import get_localzone

      local_tz = get_localzone()
      start_since = datetime.datetime(2022, 8, 8, 11, 30, 0, 0, tzinfo=local_tz).astimezone(datetime.timezone.utc)
      ```
## Auditor server

- Updating a non-existent record now returns an HTTP 404 error instead of HTTP 400 error

## Docker containers

- The `main` tag was replaced with the `edge` tag. In addition, we also now offer docker tags corresponding to releases, i.e., use the tag `0.2.0` for this release.

## HTCondor plugin

- Support for Python 3.6 and Python 3.7 has been dropped. Please move to a newer version of python.

## Development
### Update to [sqlx 0.7.1](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md)
Use this command to update the sqlx-cli to 0.7.1
- `cargo install --version=0.7.1 sqlx-cli --no-default-features --features postgres,rustls,sqlite`
