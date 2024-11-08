#!/usr/bin/env python3

import asyncio
import datetime

from tzlocal import get_localzone

from pyauditor import AuditorClientBuilder, Record


async def main():
    local_tz = get_localzone()
    print("LOCAL TIMEZONE: " + str(local_tz))

    client = await (
        AuditorClientBuilder()
        .address("127.0.0.1", 8000)
        .timeout(10)
        .send_interval(1)
        .build_queued()
    )

    print("Testing /health_check endpoint")
    health = await client.health_check()
    assert health

    print("get should not return anything because there are no records in Auditor yet")
    empty_array = await client.get()
    assert len(empty_array) == 0

    print("Adding a record to Auditor")
    record_id = "record-1"

    # datetimes sent to auditor MUST BE in UTC.
    start = datetime.datetime(
        2021, 12, 6, 16, 29, 43, 79043, tzinfo=local_tz
    ).astimezone(datetime.timezone.utc)
    record = Record(record_id, start)

    await client.add(record)
    await asyncio.sleep(2)

    print("Asserting that record in auditor db is correct")
    records = await client.get()
    assert len(records) == 1
    record = records[0]
    assert record.record_id == record_id
    assert record.start_time.replace(tzinfo=datetime.timezone.utc) == start

    print("Updating record: Adding stop time")
    stop = datetime.datetime.now(tz=local_tz).astimezone(datetime.timezone.utc)

    record = record.with_stop_time(stop)
    await client.update(record)
    await asyncio.sleep(2)

    print("Asserting that record in auditor db is correct")
    records = await client.get()
    assert len(records) == 1
    record = records[0]
    assert record.record_id == record_id
    assert record.start_time.replace(tzinfo=datetime.timezone.utc) == start
    assert record.stop_time.replace(tzinfo=datetime.timezone.utc) == stop

    print("Script test_add_update.py finished.")


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    asyncio.run(main())
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
