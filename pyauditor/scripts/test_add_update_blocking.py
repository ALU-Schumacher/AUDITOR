#!/usr/bin/env python3

from pyauditor import AuditorClientBuilder, Record
import datetime
from tzlocal import get_localzone


def main():
    local_tz = get_localzone()
    print("LOCAL TIMEZONE: " + str(local_tz))

    client = (
        AuditorClientBuilder().address("127.0.0.1", 8000).timeout(10).build_blocking()
    )

    print("Testing /health_check endpoint")
    health = client.health_check()
    assert health

    print("get should not return anything because there are no records in Auditor yet")
    empty_array = client.get()
    assert len(empty_array) == 0

    print("Adding a record to Auditor")
    record_id = "record-1"

    # datetimes sent to auditor MUST BE in UTC.
    start = datetime.datetime(
        2021, 12, 6, 16, 29, 43, 79043, tzinfo=local_tz
    ).astimezone(datetime.timezone.utc)

    record = Record(record_id, start)

    client.add(record)

    print("Asserting that record in auditor db is correct")
    records = client.get()
    assert len(records) == 1
    record = records[0]
    assert record.record_id == record_id
    assert record.start_time.replace(tzinfo=datetime.timezone.utc) == start

    print("Updating record: Adding stop time")
    stop = datetime.datetime.now(tz=local_tz).astimezone(datetime.timezone.utc)

    record = record.with_stop_time(stop)
    client.update(record)

    print("Asserting that record in auditor db is correct")
    records = client.get()
    assert len(records) == 1
    record = records[0]
    assert record.record_id == record_id
    assert record.start_time.replace(tzinfo=datetime.timezone.utc) == start
    assert record.stop_time.replace(tzinfo=datetime.timezone.utc) == stop

    print("Script test_add_update.py finished.")


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    main()
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
