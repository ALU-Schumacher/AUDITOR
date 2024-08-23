#!/usr/bin/env python3

import asyncio
import datetime

from tzlocal import get_localzone

from pyauditor import AuditorClientBuilder, Record


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

    print("Adding a records to Auditor")

    for i in range(0, 24):
        record_id = f"record-{i:02d}"

        # datetimes sent to auditor MUST BE in UTC.
        start = datetime.datetime(2022, 8, 8, i, 0, 0, 0, tzinfo=datetime.timezone.utc)
        stop = datetime.datetime(2022, 8, 9, i, 0, 0, 0, tzinfo=datetime.timezone.utc)
        record = Record(record_id, start).with_stop_time(stop)

        await client.add(record)

    print("Check if all records made it to Auditor")

    records = await client.get()
    assert len(records) == 24

    records = sorted(records, key=lambda x: x.record_id)

    for i in range(0, 24):
        assert records[i].record_id == f"record-{i:02d}"

    start_since = datetime.datetime(
        2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )

    records = await client.get_started_since(start_since)
    assert len(records) == 12

    records = sorted(records, key=lambda x: x.record_id)

    for i in range(12, 24):
        assert records[i - 12].record_id == f"record-{i:02d}"

    stop_since = datetime.datetime(
        2022, 8, 9, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )

    records = await client.get_stopped_since(stop_since)
    assert len(records) == 12

    records = sorted(records, key=lambda x: x.record_id)

    for i in range(12, 24):
        assert records[i - 12].record_id == f"record-{i:02d}"


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    asyncio.run(main())
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
