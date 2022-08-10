#!/usr/bin/env python3

import asyncio
from pyauditor import AuditorClientBuilder, Record
import datetime
import pytz
from tzlocal import get_localzone


async def main():
    local_tz = get_localzone()
    print("LOCAL TIMEZONE: " + str(local_tz))

    client = AuditorClientBuilder().address("127.0.0.1", 8000).timeout(10).build()

    print("Testing /health_check endpoint")
    health = await client.health_check()
    assert health

    print("get should not return anything because there are not records in Auditor yet")
    empty_array = await client.get()
    assert len(empty_array) == 0

    print("Adding a record to Auditor")
    record_id = "record-1"
    site_id = "site-1"
    user_id = "user-1"
    group_id = "group-1"

    # datetimes sent to auditor MUST BE in UTC.
    start = datetime.datetime(
        2021, 12, 6, 16, 29, 43, 79043, tzinfo=local_tz
    ).astimezone(pytz.utc)
    record = Record(record_id, site_id, user_id, group_id, start)

    await client.add(record)

    records = await client.get()
    assert len(records) == 1
    record = records[0]
    assert record.record_id == record_id
    assert record.site_id == site_id
    assert record.user_id == user_id
    assert record.group_id == group_id
    assert record.start_time.replace(tzinfo=pytz.utc) == start

    print("Updating record: Adding stop time")
    stop = datetime.datetime.now(tz=local_tz).astimezone(pytz.utc)

    record = record.with_stop_time(stop)
    await client.update(record)

    records = await client.get()
    assert len(records) == 1
    record = records[0]
    assert record.record_id == record_id
    assert record.site_id == site_id
    assert record.user_id == user_id
    assert record.group_id == group_id
    assert record.start_time.replace(tzinfo=pytz.utc) == start
    assert record.stop_time.replace(tzinfo=pytz.utc) == stop


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    asyncio.run(main())
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
