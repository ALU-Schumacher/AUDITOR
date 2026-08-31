#!/usr/bin/env python3

import asyncio

from pyauditor import AuditorClientBuilder


async def main():

    client = AuditorClientBuilder().address("127.0.0.1", 8000).timeout(10).build()

    records = await client.get()
    assert len(records) == 3

    records = sorted(records, key=lambda r: r.start_time)
    expected_cpu = [3, 5, 0]
    expected_cpu_milli = [2733, 4577, 3]

    for i, record in enumerate(records):
        components = list(record.components)

        assert components[0].name == "TotalCPU"
        assert components[0].amount == expected_cpu[i]

        assert components[1].name == "TotalCPU_milli"
        assert components[1].amount == expected_cpu_milli[i]


if __name__ == "__main__":
    import time

    s = time.perf_counter()
    asyncio.run(main())
