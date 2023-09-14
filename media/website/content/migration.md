+++
title = "Migration"
description = "Migration Guide"
weight = 3
+++

# From 0.1.0 to Unreleased

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

## HTCondor plugin

- Support for Python 3.6 and Python 3.7 has been dropped. Please move to a newer version of python.
