#!/usr/bin/env python3

import asyncio
from pyauditor import AuditorClientBuilder, Record
import datetime
from tzlocal import get_localzone
from pyauditor import TimeOperator, QueryBuilder, TimeValue


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

    print("Checking advanced queries")
    start_time = datetime.datetime(
        2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    value = TimeValue.set_datetime(start_time)
    operator = TimeOperator().gt(value)
    record = QueryBuilder().with_start_time(operator).build()

    records = await client.advanced_query(record)
    assert len(records) == 12

    stop_time = datetime.datetime(
        2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    value = TimeValue.set_datetime(stop_time)
    operator = TimeOperator().gt(value)
    record = QueryBuilder().with_start_time(operator).build()

    records = await client.advanced_query(record)
    assert len(records) == 12

    start_time = datetime.datetime(
        2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    stop_time = datetime.datetime(
        2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    value1 = TimeValue.set_datetime(start_time)
    value2 = TimeValue.set_datetime(stop_time)
    operator1 = TimeOperator().gt(value1)
    operator2 = TimeOperator().gt(value2)
    record = QueryBuilder().with_start_time(operator1).with_stop_time(operator2).build()

    records = await client.advanced_query(record)
    assert len(records) == 12

    start_time1 = datetime.datetime(
        2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    start_time2 = datetime.datetime(
        2022, 8, 8, 15, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    value1 = TimeValue.set_datetime(start_time1)
    value2 = TimeValue.set_datetime(start_time2)
    operator = TimeOperator().gt(value1).lt(value2)
    record = QueryBuilder().with_start_time(operator).build()

    records = await client.advanced_query(record)
    assert len(records) == 4


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    asyncio.run(main())
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
