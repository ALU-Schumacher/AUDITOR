#!/usr/bin/env python3

from pyauditor import Record, Component, Score
import datetime
import pytz
from tzlocal import get_localzone


def main():
    local_tz = get_localzone()
    print("LOCAL TIMEZONE: " + str(local_tz))

    record_id = "record-1"
    site_id = "site-1"
    user_id = "user-1"
    group_id = "group-1"
    score = "score-1"
    factor = 12.0
    component = "component-1"
    amount = 21

    # datetimes sent to auditor MUST BE in UTC.
    start = datetime.datetime(
        2021, 12, 6, 16, 29, 43, 79043, tzinfo=local_tz
    ).astimezone(pytz.utc)

    score1 = Score(score, factor)
    score2 = Score(score, factor)
    assert score1 == score2

    comp1 = Component(component, amount)
    comp2 = Component(component, amount)
    assert comp1 == comp2

    comp1 = Component(component, amount).with_score(score1)
    comp2 = Component(component, amount).with_score(score2)
    assert comp1 == comp2

    record1 = Record(record_id, site_id, user_id, group_id, start)
    record2 = Record(record_id, site_id, user_id, group_id, start)
    assert record1 == record2

    record1 = Record(record_id, site_id, user_id, group_id, start).with_component(comp1)
    record2 = Record(record_id, site_id, user_id, group_id, start).with_component(comp1)
    assert record1 == record2


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    main()
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
