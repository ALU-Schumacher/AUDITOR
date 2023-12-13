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

   from pyauditor import Record, Meta, Component, Score

   # Define meta information
   record_id = "record-1" # Must be unique for all records in Auditor!

   # Time when the resource became available
   start = datetime.datetime(2021, 12, 6, 16, 29, 43, 79043) # in UTC

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
   stop = datetime.datetime(2021, 12, 6, 18, 0, 0, 79043) # in UTC
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


Updating records in Auditor
===========================

Auditor accepts incomplete records. In particular, the stop time can be missing. These records can be updated at a later time, by adding the same record which includes a stop time.
Note that the ``record_id`` must match the one already in the database! 
Fields other than the stop time cannot be updated.


.. code-block:: python

   await client.add(record.with_stop_time(stop_time))


Receiving all records from Auditor
==================================

Via ``get()`` all records can be retrieved from Auditor:

.. code-block:: python

   list_of_records = await client.get()


Receiving all records started/stopped since a given timestamp
=============================================================

The records to be retrieved can be limited to the ones started or stopped since a given timestamp.

.. code-block:: python

   list_of_records_started_since = await client.get_started_since(timestamp)
   list_of_records_stopped_since = await client.get_stopped_since(timestamp)


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

When the timestamps you are using are already in UTC, they can be used without further processing.

.. code-block:: python

   timestamp = datetime.datetime(2022, 8, 16, 12, 00, 43, 48942)


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
