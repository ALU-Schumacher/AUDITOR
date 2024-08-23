#!/usr/bin/env python3

import datetime

from tzlocal import get_localzone

from pyauditor import AuditorClientBuilder, Component, Meta, Record


def main():
    local_tz = get_localzone()
    print("LOCAL TIMEZONE: " + str(local_tz))

    client = (
        AuditorClientBuilder().address("127.0.0.1", 8000).timeout(10).build_blocking()
    )

    print("Testing /health_check endpoint")
    health = client.health_check()
    assert health

    print("get should not return anything because there are not records in Auditor yet")
    empty_array = client.get()
    assert len(empty_array) == 0

    print("Adding a record to Auditor")
    record_id = "record-1"

    # datetimes sent to auditor MUST BE in UTC.
    start = datetime.datetime(
        2021, 12, 6, 16, 29, 43, 79043, tzinfo=local_tz
    ).astimezone(datetime.timezone.utc)
    # stop = datetime.datetime(2022, 8, 9, 3, 0, 0, 0, tzinfo=datetime.timezone.utc)
    component1 = Component("comp-1", 10)
    meta = (
        Meta()
        .insert("site_id", ["site_A"])
        .insert("group_id", ["group_1"])
        .insert("nodes", ["node1", "node2"])
    )
    record = Record(record_id, start).with_component(component1).with_meta(meta)

    client.add(record)

    records = client.get()
    assert len(records) == 1
    record = records[0]
    assert record.record_id == record_id
    assert record.start_time.replace(tzinfo=datetime.timezone.utc) == start
    assert record.components[0].name == "comp-1"
    assert record.components[0].amount == 10
    assert record.meta.get("site_id") == ["site_A"]
    assert record.meta.get("group_id") == ["group_1"]
    assert record.meta.get("nodes") == ["node1", "node2"]


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    main()
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
