#!/usr/bin/env python3

import asyncio
import datetime

from tzlocal import get_localzone

from pyauditor import (
    AuditorClientBuilder,
    Component,
    Meta,
    Operator,
    QueryBuilder,
    Record,
    Score,
    Value,
)


async def main():
    local_tz = get_localzone()
    print("LOCAL TIMEZONE: " + str(local_tz))

    client = (
        AuditorClientBuilder()
        .address("localhost", 8443)
        .timeout(10)
        .with_tls(
            "scripts/certs/rootCA.pem",
            "scripts/certs/client-cert.pem",
            "scripts/certs/client-key.pem",
        )
        .build()
    )

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
        meta = (
            Meta()
            .insert("site_id", ["site_A"])
            .insert("group_id", ["group_1"])
            .insert("nodes", ["node1", "node2"])
        )
        score1 = Score("HEPSPEC", 1.0)
        component1 = Component("comp-1", 10).with_score(score1)

        record = (
            Record(record_id, start)
            .with_stop_time(stop)
            .with_meta(meta)
            .with_component(component1)
        )

        await client.add(record)

    for i in range(0, 10):
        record_id = f"record2-{i:02d}"

        # datetimes sent to auditor MUST BE in UTC.
        start = datetime.datetime(2023, 8, 8, i, 0, 0, 0, tzinfo=datetime.timezone.utc)
        stop = datetime.datetime(2023, 8, 9, i, 0, 0, 0, tzinfo=datetime.timezone.utc)
        meta = (
            Meta()
            .insert("site_id", ["site_B"])
            .insert("group_id", ["group_2"])
            .insert("nodes", ["node1", "node2"])
        )
        score2 = Score("HEPSPEC", 1.0)
        component2 = Component("comp-2", 8).with_score(score2)

        record = (
            Record(record_id, start)
            .with_stop_time(stop)
            .with_meta(meta)
            .with_component(component2)
        )

        await client.add(record)

    print("Check if all records made it to Auditor")

    all_records = await client.get()
    assert len(all_records) == 34

    print("Checking advanced queries")
    start_time = datetime.datetime(
        2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    value = Value.set_datetime(start_time)
    operator = Operator().gt(value)
    query_string = QueryBuilder().with_start_time(operator).build()

    records = await client.advanced_query(query_string)
    assert len(records) == 22


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    asyncio.run(main())
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
