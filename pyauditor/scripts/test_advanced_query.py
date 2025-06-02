#!/usr/bin/env python3

import asyncio
import datetime

from tzlocal import get_localzone

from pyauditor import (
    AuditorClientBuilder,
    Component,
    ComponentQuery,
    Meta,
    MetaOperator,
    MetaQuery,
    Operator,
    QueryBuilder,
    Record,
    Score,
    SortBy,
    Value,
)


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

    stop_time = datetime.datetime(
        2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    value = Value.set_datetime(stop_time)
    operator = Operator().gt(value)
    query_string = QueryBuilder().with_start_time(operator).build()

    records = await client.advanced_query(query_string)
    assert len(records) == 22

    start_time = datetime.datetime(
        2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    stop_time = datetime.datetime(
        2022, 8, 9, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    value1 = Value.set_datetime(start_time)
    value2 = Value.set_datetime(stop_time)
    operator1 = Operator().gt(value1)
    operator2 = Operator().gt(value2)
    query_string = (
        QueryBuilder().with_start_time(operator1).with_stop_time(operator2).build()
    )

    records = await client.advanced_query(query_string)
    assert len(records) == 22

    start_time1 = datetime.datetime(
        2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    start_time2 = datetime.datetime(
        2022, 8, 8, 15, 30, 0, 0, tzinfo=datetime.timezone.utc
    )
    value1 = Value.set_datetime(start_time1)
    value2 = Value.set_datetime(start_time2)
    operator = Operator().gt(value1).lt(value2)
    query_string = QueryBuilder().with_start_time(operator).build()

    records = await client.advanced_query(query_string)
    assert len(records) == 4

    meta_operator = MetaOperator().contains(["group_1"])
    meta_query = MetaQuery().meta_operator("group_id", meta_operator)
    query_string = QueryBuilder().with_meta_query(meta_query).build()

    records = await client.advanced_query(query_string)
    record = records[0]
    assert record.meta.get("group_id") == ["group_1"]
    assert len(records) == 24

    meta_operator = MetaOperator().contains(["group_2"])
    meta_query = MetaQuery().meta_operator("group_id", meta_operator)
    records = QueryBuilder().with_meta_query(meta_query).build()

    records = await client.advanced_query(records)
    assert len(records) == 10

    meta_operator = MetaOperator().contains(["placeholder"])
    meta_query = MetaQuery().meta_operator("group_id", meta_operator)
    records = QueryBuilder().with_meta_query(meta_query).build()

    records = await client.advanced_query(records)
    assert len(records) == 0

    value = Value.set_count(10)
    component_operator = Operator().equals(value)
    component_query = ComponentQuery().component_operator("comp-1", component_operator)
    query_string = QueryBuilder().with_component_query(component_query).build()

    records = await client.advanced_query(query_string)
    record = records[0]
    assert record.components[0].name == "comp-1"
    assert record.components[0].amount == 10
    assert record.components[0].scores[0].name == "HEPSPEC"
    assert record.components[0].scores[0].value == 1.0
    assert len(records) == 24

    value = Value.set_count(8)
    component_operator = Operator().equals(value)
    component_query = ComponentQuery().component_operator("comp-2", component_operator)
    query_string = QueryBuilder().with_component_query(component_query).build()

    records = await client.advanced_query(query_string)
    record = records[0]
    assert record.components[0].name == "comp-2"
    assert record.components[0].amount == 8
    assert record.components[0].scores[0].name == "HEPSPEC"
    assert record.components[0].scores[0].value == 1.0
    assert len(records) == 10

    value = Value.set_count(8)
    component_operator = Operator().equals(value)
    component_query = ComponentQuery().component_operator(
        "placeholder", component_operator
    )
    query_string = QueryBuilder().with_component_query(component_query).build()

    records = await client.advanced_query(query_string)
    assert len(records) == 0

    sort_by = SortBy().descending("start_time")
    query_string = QueryBuilder().sort_by(sort_by).build()

    records = await client.advanced_query(query_string)
    assert len(records) == 34

    for i in range(0, 10):
        assert records[i].record_id == f"record2-{9 - i:02d}"

    query_string = QueryBuilder().limit(4).build()

    records = await client.advanced_query(query_string)
    assert len(records) == 4

    query_string = QueryBuilder().with_record_id(f"record-{3:02d}").build()

    records = await client.advanced_query(query_string)
    assert len(records) == 1


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    asyncio.run(main())
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
