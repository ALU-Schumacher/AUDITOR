.. _ref_examples:

========
Tutorial
========

This section walks you through several basic usecases of `pyauditor`.

.. warning::
   Records and interacting with Auditor requires timestamps, for instance to indicate when a resource became available or unavailable, or when requesting a list of records from Auditor.
   For simplicity, the entire Auditor ecosystem uses UTC.
   Pythons handling of timezones is unfortunately suboptimal, therefore ensuring that the times are correct is your responsibility.
   At the end of this section there is a short tutorial on how to get correct timestamps.

Auditor is designed around so-called records, which are the unit of accountable resources.
Records are created and pushed to Auditor, which stores them in a database.
These records can then be requested again from Auditor to take an action based on the information stored in the records.

A record consists of a unique identifier and meta information which provides some context (associated site, group, user).
It also contains an arbitrary number of `components` that are to be accounted for (CPU, RAM, Disk, ...) and the amount of each of these components.
The components can optionally be enhanced with `scores`, which are floating point values which put components of the same kind, but different performance in relation to each other.
For instance, in case of CPUs these could be HEPSPEC06 benchmark values.

pyauditor is an async library and as such requires an event loop.

Creating a Record
=================


.. code-block:: python

   from pyauditor import Record, Meta, Component, score

   # Define meta information
   record_id = "record-1" # Must be unique for all records in Auditor!

   # Time when the resource became available
   start = datetime.datetime(2021, 12, 6, 16, 29, 43, 79043, tzinfo=datetime.timezone.utc) # in UTC

   # Create record
   record = Record(record_id, start)

   # Create a component (10 CPU cores)
   component = Component("CPU", 10)

   # Create a score
   score = Score("HEPSPEC06", 9.2)

   # Attach the score to the component
   component = component.with_score(score)

   # Attach the component to the record
   record = record.with_component(component)

   # Create meta 
   meta = Meta()

   # Add information to meta
   meta = meta.insert("site_id", ["site1"])

   # Attach meta to record
   record = record.with_meta(meta)

   # Resource stopped being available
   stop = datetime.datetime(2021, 12, 6, 18, 0, 0, 79043, tzinfo=datetime.timezone.utc) # in UTC
   
   record = record.with_stop_time(stop)



Connecting to Auditor
=====================

The ``AuditorClientBuilder`` is used to build an ``AuditorClient`` object which can be used for interacting with Auditor:

.. code-block:: python

   from pyauditor import AuditorClientBuilder

   # Create the builder
   builder = AuditorClientBuilder()

   # Configure the Builder
   auditor_address = "127.0.0.1"
   auditor_port = 8000
   builder = builder.address(auditor_address, auditor_port)

   # Build the AuditorClient object
   client = builder.build()



Pushing records to Auditor
==========================

Assuming that a record and a client were already created, the record can be pushed to Auditor like this:

.. code-block:: python

   await client.add(record)

Pushing a list of records to Auditor
==================================

Assuming that a list of records and a client were already created, the record can be pushed to Auditor like this:

.. code-block:: python

   await client.bulk_insert(records)

Updating records in Auditor
===========================

Auditor accepts incomplete records. In particular, the stop time can be missing. These records can be updated at a later time, by adding the same record which includes a stop time.
Note that the ``record_id`` must match the one already in the database! 
Fields other than the stop time cannot be updated.


.. code-block:: python

   await client.add(record.with_stop_time(stop_time))


Receiving all records from Auditor (Deprecated)
===============================================

Via ``get()`` all records can be retrieved from Auditor:

.. code-block:: python

   list_of_records = await client.get()


Receiving all records started/stopped since a given timestamp (Deprecated)
==========================================================================

The records to be retrieved can be limited to the ones started or stopped since a given timestamp.

.. code-block:: python

   list_of_records_started_since = await client.get_started_since(timestamp)
   list_of_records_stopped_since = await client.get_stopped_since(timestamp)


Advanced Query
==============
Records can be queried using fields and operators. 

Template Query
--------------

```
GET /records?<field>[<operator>]=<value>
```

This is how the query is structured and multiple fields and values can be queried together as shown below.


Operators
---------

- `gt` (greater than)
- `gte` (greater than or equal to)
- `lt` (less than)
- `lte` (less than or equal to)
- `equals` (equal to)

Meta Operators
--------------

- `c` (contains)
- `dnc` (does not contain)

SortBy Operators
----------------
 - `asc` (ascending order)
 - `desc` (descending order)

 
SortBy Column names
-------------------
You can specify the column on which the sorting must happen
The following columns are supported for sortby option
- `start_time`
- `stop_time`
- `runtime`
- `record_id`

Filter Fields and Operators
---------------------------

The table shows the fields and the corresponding operators available for each field with which a query can be built.

+--------------+-----------------------------------------------+----------------------------------------+---------------------------------------------+
| Field        | Description                                   | Operators                              | Examples (query representation)             |
+==============+===============================================+========================================+=============================================+
| `record_id`  | Exact record to be retrieved using record_id  |                                        | record_id=<record_id>                       |
+--------------+-----------------------------------------------+----------------------------------------+---------------------------------------------+
| `start_time` | Start time of the event (`DateTime<Utc>`)     | `gt`, `gte`, `lt`, `lte`               | start_time[gt]=<timestamp>                  |
+--------------+-----------------------------------------------+----------------------------------------+---------------------------------------------+
| `stop_time`  | Stop time of the event (`DateTime<Utc>`)      | `gt`, `gte`, `lt`, `lte`               | stop_time[gt]=<timestamp>                   |
+--------------+-----------------------------------------------+----------------------------------------+---------------------------------------------+
| `runtime`    | Runtime of the event (in seconds)             | `gt`, `gte`, `lt`, `lte`               | runtime[gt]=<int>                           |
+--------------+-----------------------------------------------+----------------------------------------+---------------------------------------------+
| `meta`       | Meta information                              | `c`, `dnc`                             | meta[<meta_key>][c][0]=<meta_value>         |
+--------------+-----------------------------------------------+----------------------------------------+---------------------------------------------+
| `component`  | Component identifier                          | `gt`, `gte`, `lt`, `lte`, `equals`     | component[<component_name>][gt]=<amount>    |
+--------------+-----------------------------------------------+----------------------------------------+---------------------------------------------+
| `sort_by`    | Sort the records                              | `asc`, `desc`                          | sort_by[asc]=<column_name>                  |
+--------------+-----------------------------------------------+----------------------------------------+---------------------------------------------+
| `limit`      | Limit the query results                       |                                        | limit=<number>                              |
+--------------+-----------------------------------------------+----------------------------------------+---------------------------------------------+


Meta field can be used to query records by specifying the meta key and MetaOperator must be used
to specify meta values. The MetaOperator must be used to specify whether the value is
contained or is not contained for the specific Metakey.

Component field can be used to query records by specifying the component name (CPU) and ['Operator'] must be used
to specify the amount. 

To query records based on a range, specify the field with two operators
Either with gt or gte and lt or lte.

For example,
To query records with start_time ranging between two timestamps.

```text
Get records?start_time[gt]=timestamp1&start_time[lt]=timestamp2
```


QueryBuilder
============
Below are the examples to query records using QueryBuilder methods. It helps to build query string which can be passed
as an argument to advanced_query function to get the records.

Examples 1:
-----------
Query all records

.. code-block:: python

    from pyauditor import QueryBuilder

    query_string = QueryBuilder().build()
    records = await client.advanced_query(query_string)

Example 2:
----------
Query records with start_time greater than the timestamp

.. code-block:: python

    from pyauditor import Value, Operator, QueryBuilder

    # Set the datetime value in Utc using Value object
    value = Value.set_datetime(timestamp)

    # Set the operator using Value object created in the previous step
    operator = Operator().gt(value)

    # Build the query string using build method from QueryBuilder object
    query_string = QueryBuilder().with_start_time(operator).build()

    # Pass the query_string as an argument to advanced_query function
    records = await client.advanced_query(query_string)

Example 3:
----------
Query records with meta key = site_id and value = site1

.. code-block:: python

    from pyauditor import MetaOperator, MetaQuery, QueryBuilder

    meta_operator = MetaOperator().contains("[site1]")
    meta_query = MetaQuery().meta_operator("site_id", meta_operator)
    query_string = QueryBuilder().with_meta_query(meta_query).build()
    records = await client.advanced_query(query_string)

Example 4:
----------
Query records with component name = CPU and amount = 10

.. code-block:: python

    from pyauditor import Operator, ComponentQuery, Value, QueryBuilder

    value = Value.set_count(10)
    component_operator = Operator().equals(value)
    component_query = ComponentQuery().component_operator("CPU", component_operator)
    query_string = QueryBuilder().with_component_query(component_query).build()
    records = await client.advanced_query(query_string)

Example 5:
----------
Query records sorted by stop_time in descending order and limit the query to 500 records

.. code-block:: python

    from pyauditor import QueryBuilder, SortBy

    sort_by = SortBy().descending("stop_time")
    query_string = QueryBuilder().sort_by(sort_by).limit(500).build()
    records = await client.advanced_query(query_string)

Example 5:
----------
Query records by exact record_id

.. code-block:: python

    from pyauditor import QueryBuilder

    query_string = QueryBuilder().with_record_id("record-1").build()
    records = await client.advanced_query(query_string)

Checking the health of Auditor
==============================

The health of Auditor can be checked with

.. code-block:: python

   healthy = await client.health_check()
   if healthy:
       print(":)")
   else:
       print(":(")


Creating UTC timestamps
=======================

This section gives hints on how to create appropriate timestamps for use with Auditor.
The `actual` timezone assigned to the `datetime` object is irrelevant when passed to any pyauditor classes/functions/methods!
Only the actual numbers for hours, minutes, and so on matter.


Timestamp already in UTC
------------------------

Even if the timestamps you are using are already in UTC, timezone information should be explicitly specified.

.. code-block:: python

   timestamp = datetime.datetime(2021, 12, 6, 16, 29, 43, 79043, tzinfo=datetime.timezone.utc) # in UTC



Timestamp in local time
-----------------------

This requires the python modules ``tzlocal``.

Assuming that you are creating the timestamp yourself (it is not obtained from an external source), you need to attach the local timezone to the timestamp and then convert it to UTC:

.. code-block:: python

   from tzlocal import get_localzone
   local_tz = get_localzone()
   timestamp = datetime.datetime(2022, 8, 16, 12, 00, 43, 48942, tzinfo=local_tz).astimezone(datetime.timezone.utc)

If you have a ``datetime`` object from some external source, the timezone can be attached like this:


.. code-block:: python

   from tzlocal import get_localzone
   local_tz = get_localzone()
   timestamp = datetime_from_somewhere_else.replace(tzinfo=local_tz).astimezone(datetime.timezone.utc)


When using ``datetime.now()`` the local timezone also has to be provided explicitly. However, the parameter is now called ``tz`` instead of ``tzinfo`` because who needs consistency anyways?

.. code-block:: python

   from tzlocal import get_localzone
   local_tz = get_localzone()
   timestamp = datetime.now(tz=local_tz).astimezone(datetime.timezone.utc)
