+++
title = "Migration"
description = "Migration Guide"
weight = 3
+++

# From 0.3.0/0.3.1 to 0.4.0

## Development

### Update to [sqlx 0.7.3](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md)
Use this command to update the sqlx-cli to 0.7.3
- `cargo install --version=0.7.3 sqlx-cli --no-default-features --features postgres,rustls,sqlite`

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
