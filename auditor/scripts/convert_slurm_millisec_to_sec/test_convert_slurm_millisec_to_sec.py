#!/usr/bin/env python3

import asyncio

from pyauditor import AuditorClientBuilder


async def main():

    client = AuditorClientBuilder().address("127.0.0.1", 8000).timeout(10).build()

    records = await client.get()
    assert len(records) == 2

    for record in records:
        list_of_components = list(record.components)

        assert list_of_components[0].name == "TotalCPU"

        assert list_of_components[0].amount == 0

        assert list_of_components[1].name == "TotalCPU_milli"

        assert list_of_components[1].amount == 273


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    asyncio.run(main())
