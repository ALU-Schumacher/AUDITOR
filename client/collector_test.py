#!/usr/bin/env python3
"""Test collector"""

from __future__ import annotations  # not necessary in 3.10
import asyncio
import logging
from uuid import uuid4
from auditorclient.record import Record, Components, Scores
from auditorclient.client import AuditorClient
from pprint import pprint


async def main(client: AuditorClient):
    await client.start()

    #  components = Components().add_component("CPU2", 1, 1.3)
    #
    #  record_id = f"{uuid4().int}"
    #  record = Record(record_id, "nemo", "sch", "atlsch", components).with_start_time(
    #      "2019-11-28T12:45:59.324310806Z"
    #  )

    components = (
        Components()
        .add_component("Cores", 40, Scores().add_score("score1", 1.0))
        .add_component("Memory", 100)
        .add_component("Disk", 196)
    )
    record_id = "nemo-13312889"
    record = Record(record_id, "NEMO", "atlsch", "atlsch", components).with_start_time(
        "2022-02-28T07:20:01.104919+00:00"
    )
    # {'job_id': 'nemo-13312889', 'site_id': 'NEMO', 'user_id': 'atlsch', 'group_id': 'atlsch', 'components': [{'name': 'Cores', 'amount': 40, 'factor': 1}, {'name': 'Memory', 'amount': 100, 'factor': 1}, {'name': 'Disk', 'amount': 196, 'factor': 1}], 'start_time': '2022-02-28T07:20:01.104919+00:00', 'stop_time': None}

    print(f"Submitting {record}")
    await client.add_record_queue(record)
    print("Done")
    #
    #  await asyncio.sleep(1)
    #
    #  #  try:
    #  #      response = await client.add_record(record)
    #  #      print(response.status)
    #  #  except RecordExistsError as e:
    #  #      print(f"Record {e.record_id} on site {e.site_id} already exists!")
    #
    record.with_stop_time("2022-11-29T12:45:59.324310806Z")
    #  #  response = await client.update_record(record)
    #  #  print(response)
    response = await client.update_record_queue(record)
    print(response)

    #  for i in range(100000):
    #      record = (
    #          Record(f"maufb-{i}", "nemo", "sch", "atlsch", components)
    #          .with_start_time("2019-11-28T12:45:59.324310806Z")
    #          .with_stop_time("2020-11-29T12:45:59.324310806Z")
    #      )
    #      response = await client.add_record_queue(record)

    #  response = await client.get_since("2021-05-28T12:00:59.324310806Z")
    #  pprint(response)

    #  await asyncio.sleep(2)

    #  response = await client.get()
    #  pprint(response)

    #  while True:
    #      await asyncio.sleep(0.5)


if __name__ == "__main__":
    #  logging.basicConfig(filename="example.log", encoding="utf-8", level=logging.DEBUG)
    #  logging.basicConfig(encoding="utf-8", level=logging.DEBUG)
    logging.basicConfig(level=logging.DEBUG)
    #  logging.basicConfig(level=logging.INFO)

    loop = asyncio.get_event_loop()
    #  loop.set_debug(True)
    client = AuditorClient("127.0.0.1", 8000, num_workers=4)
    # client = AuditorClient("127.0.0.1", 8000, num_workers=1, db=None)
    try:
        loop.run_until_complete(main(client))
    except KeyboardInterrupt:
        pass
    finally:
        loop.run_until_complete(client.stop())
    loop.close()
