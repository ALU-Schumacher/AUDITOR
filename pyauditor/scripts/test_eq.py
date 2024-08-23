#!/usr/bin/env python3

import datetime

from tzlocal import get_localzone

from pyauditor import Component, Record, Score


def main():
    local_tz = get_localzone()
    print("LOCAL TIMEZONE: " + str(local_tz))

    record_id = "record-1"
    score = "score-1"
    value = 12.0
    component = "component-1"
    amount = 21

    # datetimes sent to auditor MUST BE in UTC.
    start = datetime.datetime(
        2021, 12, 6, 16, 29, 43, 79043, tzinfo=local_tz
    ).astimezone(datetime.timezone.utc)

    score1 = Score(score, value)
    score2 = Score(score, value)
    assert score1 == score2

    comp1 = Component(component, amount)
    comp2 = Component(component, amount)
    assert comp1 == comp2

    comp1 = Component(component, amount).with_score(score1)
    comp2 = Component(component, amount).with_score(score2)
    assert comp1 == comp2

    record1 = Record(record_id, start)
    record2 = Record(record_id, start)
    assert record1 == record2

    record1 = Record(record_id, start).with_component(comp1)
    record2 = Record(record_id, start).with_component(comp1)
    assert record1 == record2


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    main()
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
