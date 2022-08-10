#!/usr/bin/env python3

import asyncio
from pyauditor import AuditorClientBuilder, Record, Component, Score
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
    stop = datetime.datetime(2022, 8, 9, 3, 0, 0, 0, tzinfo=pytz.utc)
    score1 = Score("HEPSPEC", 1.0)
    score2 = Score("OTHERSPEC", 4.0)
    component1 = Component("comp-1", 10).with_score(score1)
    component2 = Component("comp-2", 100).with_score(score1).with_score(score2)
    record = (
        Record(record_id, site_id, user_id, group_id, start)
        .with_component(component1)
        .with_component(component2)
    )

    await client.add(record)

    records = await client.get()
    assert len(records) == 1
    record = records[0]
    assert record.record_id == record_id
    assert record.site_id == site_id
    assert record.user_id == user_id
    assert record.group_id == group_id
    assert record.start_time.replace(tzinfo=pytz.utc) == start
    assert record.components[0].name == "comp-1"
    assert record.components[0].amount == 10
    assert record.components[0].scores[0].name == "HEPSPEC"
    assert record.components[0].scores[0].factor == 1.0

    assert record.components[1].name == "comp-2"
    assert record.components[1].amount == 100
    assert record.components[1].scores[0].name == "HEPSPEC"
    assert record.components[1].scores[0].factor == 1.0
    assert record.components[1].scores[1].name == "OTHERSPEC"
    assert record.components[1].scores[1].factor == 4.0


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    asyncio.run(main())
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
