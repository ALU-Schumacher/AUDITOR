#!/usr/bin/env python3
"""Test plugin"""

import asyncio
import logging
from auditorclient.client import AuditorClient
from pprint import pprint
from dateutil import parser
from math import floor

#  from datetime import timedelta


def compute_cputime_per_group(response):
    data = {}
    for rec in response:
        if rec["stop_time"]:
            group_id = rec["group_id"]
            if group_id not in data:
                data[group_id] = {"count": 0, "cpu_time": 0.0}
            data[group_id]["count"] += 1
            print(rec["start_time"])
            print(rec["stop_time"])
            data[group_id]["cpu_time"] += (
                (parser.parse(rec["stop_time"]) - parser.parse(rec["start_time"]))
                * rec["components"][0]["amount"]
                * rec["components"][0]["factor"]
            ).total_seconds()
    return data


def compute_priority_per_group(cputime_per_group, max_priority=65533):
    max_cputime = max([v["cpu_time"] for v in cputime_per_group.values()])
    return {
        k: v["cpu_time"] * (float(max_priority) / float(max_cputime))
        for k, v in cputime_per_group.items()
    }


def construct_commands(priorities, group_to_partition_mapping):
    return [
        f"sudo scontrol update PartitionName={group_to_partition_mapping[group]} "
        + f"PriorityJobFactor={int(floor(priority))}"
        for group, priority in priorities.items()
    ]


async def main(client: AuditorClient):
    await client.start()

    #  response = await client.get_since("2021-05-28T12:00:59.324310806Z")
    #  pprint(response)

    response = await client.get()
    #  pprint(response)

    data = compute_cputime_per_group(response)
    for (k, v) in data.items():
        print(f"{k}: {v['count']} | {v['cpu_time']}")

    prio = compute_priority_per_group(data)
    pprint(prio)

    group_to_partition_mapping = {
        "group1": "no_partition_1",
        "group2": "no_partition_2",
        "atlsch": "nemo_vm_atlsch",
        "atljak": "nemo_vm_atljak",
        "atlher": "nemo_vm_atlher",
        "atlhei": "nemo_vm_atlhei",
    }

    pprint(construct_commands(prio, group_to_partition_mapping))

    #  while True:
    #      response = await client.get()
    #      pprint(response)
    #      await asyncio.sleep(5)


if __name__ == "__main__":
    #  logging.basicConfig(filename="example.log", level=logging.DEBUG)
    #  logging.basicConfig(level=logging.DEBUG)
    logging.basicConfig(level=logging.INFO)

    loop = asyncio.get_event_loop()
    #  loop.set_debug(True)
    #  client = AuditorClient("127.0.0.1", 8000, num_workers=4)
    client = AuditorClient("127.0.0.1", 8000, num_workers=1, db=None)
    try:
        loop.run_until_complete(main(client))
    except KeyboardInterrupt:
        pass
    finally:
        loop.run_until_complete(client.stop())
    loop.close()
