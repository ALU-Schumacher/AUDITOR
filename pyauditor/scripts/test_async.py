#!/usr/bin/env python3
# countasync.py

import asyncio
from pyauditor import AuditorClientBuilder, Record
import datetime


async def main():
    blah = AuditorClientBuilder().address("127.0.0.1", 8000).timeout(100).build()
    fu = await blah.health_check()
    print(fu)
    fu = await blah.get()
    print(fu)
    start = datetime.datetime(2021, 12, 6, 16, 29, 43, 79043);
    record = Record("name", "site", "user", "group", start)
    await blah.add(record)
    fu = await blah.get()
    print(fu)
    #  await asyncio.gather(count(), count(), count())

if __name__ == "__main__":
    import time
    s = time.perf_counter()
    asyncio.run(main())
    elapsed = time.perf_counter() - s
    print(f"{__file__} executed in {elapsed:0.2f} seconds.")
