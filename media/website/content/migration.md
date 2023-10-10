+++
title = "Migration"
description = "Migration Guide"
weight = 3
+++

# From 0.2.0 to unreleased
## Priority plugin
The config file must contain an 'auditor' field which comprises of addr and port number. 
Prometheus configuration is optional. Metrics are exported according to the frequency specified (in seconds). 

- Before 
	```
	addr: "localhost"
	port: 8000
	components:
	  NumCPUs: "cpu-1"
	min_priority: 1
	max_priority: 65335
	group_mapping:
	  group1: 
	    - "part-1"
	  group2:
	    - "part-2"
	  group3:
	    - "part-3"
	  group4:
	    - "part-4"
	commands:
	  - "/usr/bin/scontrol update PartitionName={1} PriorityJobFactor={priority}"
	  - "echo '{group}: {priority}'"
	```

 - After
	 ``` 
	auditor:
	   addr: "localhost"
	   port: 8000
	components:
	   NumCPUs: "cpu-1"
	min_priority: 1
	max_priority: 65335
	group_mapping:
       group1: 
	     - "part-1"
	   group2:
	     - "part-2"
	   group3:
	     - "part-3"
	   group4:
	     - "part-4"
	commands:
	   - "/usr/bin/scontrol update PartitionName={1} PriorityJobFactor={priority}"
	   - "echo '{group}: {priority}'"
	prometheus:
	   enable: true
	   addr: "localhost"
	   port: 9000
	   frequency: 3600
	   metrics:
	     - ResourceUsage
	     - Priority
	 ```


## AUDITOR
### Standardized REST APIs

Auditor REST APIs are changed as shown in the table below. 

| Before                                                 	| After                                                            	|
|--------------------------------------------------------	|------------------------------------------------------------------	|
| /health_check (GET)                                    	| /health_check (GET)                                              	|
| /add (POST) -> add record                              	| /record (POST) -> add record                                     	|
| /update (POST) -> update record                        	| /record (PUT) -> update record                                   	|
| /get (GET) -> get all records                          	| /record (GET) -> get all records                                 	|
| /get/[started/stopped]/since/{date} (GET) -> get since 	| /record?state=[started/stopped]&since={date}  (GET) -> get since 	|

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
