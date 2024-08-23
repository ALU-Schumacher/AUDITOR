#!/usr/bin/env python3

import asyncio
import datetime

from tzlocal import get_localzone

from pyauditor import AuditorClientBuilder, Component, Record, Score


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

    print("get should not return anything because there are not records in Auditor yet")
    empty_array = await client.get()
    assert len(empty_array) == 0

    print("Adding a record to Auditor")
    record_id = "record-1"

    # datetimes sent to auditor MUST BE in UTC.
    start = datetime.datetime(
        2021, 12, 6, 16, 29, 43, 79043, tzinfo=local_tz
    ).astimezone(datetime.timezone.utc)
    # stop = datetime.datetime(2022, 8, 9, 3, 0, 0, 0, tzinfo=datetime.timezone.utc)
    score1 = Score("HEPSPEC", 1.0)
    score2 = Score("OTHERSPEC", 4.0)
    component1 = Component("comp-1", 10).with_score(score1)
    component2 = Component("comp-2", 100).with_score(score1).with_score(score2)
    record = (
        Record(record_id, start).with_component(component1).with_component(component2)
    )

    await client.add(record)
    await asyncio.sleep(2)

    records = await client.get()
    assert len(records) == 1
    record = records[0]
    assert record.record_id == record_id
    assert record.start_time.replace(tzinfo=datetime.timezone.utc) == start
    assert record.components[0].name == "comp-1"
    assert record.components[0].amount == 10
    assert record.components[0].scores[0].name == "HEPSPEC"
    assert record.components[0].scores[0].value == 1.0

    assert record.components[1].name == "comp-2"
    assert record.components[1].amount == 100
    assert record.components[1].scores[0].name == "HEPSPEC"
    assert record.components[1].scores[0].value == 1.0
    assert record.components[1].scores[1].name == "OTHERSPEC"
    assert record.components[1].scores[1].value == 4.0


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    asyncio.run(main())
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
