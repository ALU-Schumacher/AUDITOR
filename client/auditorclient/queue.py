import logging
import asyncio
from datetime import datetime
from .db import DB, DBsqlite
from .task import Task


class Queue:
    def __init__(self, sleep_time: float = 0.1, db: DB = DBsqlite()):
        # PriorityQueue is used here instead of Queue because with this we can enforce
        # ordering in the processing of the events. In particular, `ADD` events are
        # processed before `UPDATE` events to avoid problems when an `ADD` fails due to
        # network issues and gets requeued AFTER the corresponding `UPDATE` event. In
        # such a case the `UPDATE` would get lost, while the `ADD` will be executed
        # later on. This can cause inconsistencies/incomplete information in the
        # auditor database.
        self._queue = None
        self._sleep_time = sleep_time
        self._db = db
        self._logger = logging.getLogger("auditorclient.queue.Queue")

    async def start(self):
        self._queue = asyncio.PriorityQueue()
        if self._db is not None:
            self._logger.debug("Connecting to database")
            await self._db.start()
            tasks = await self._db.get_all()
            for task in tasks:
                self._logger.debug(f"Restored task from database: {task}")
                await self._queue.put(task)

    async def get(self) -> Task:
        while True:
            task = await self._queue.get()
            #  self._logger.debug(f"Got task from queue: {task}")
            schedule_after = task.schedule_after()
            if schedule_after is None or datetime.now() > schedule_after:
                self._logger.debug(f"Returning task: {task}")
                task.wait_for_sec(None)
                if self._db:
                    await self._db.delete(task)
                return task
            else:
                #  self._logger.debug(f"Task cannot be scheduled yet: {task}")
                await asyncio.sleep(self._sleep_time)
                self._queue.task_done()
                await self._queue.put(task)

    async def put(self, task: Task, wait_for_sec: int = None) -> None:
        if self._db is not None:
            await self._db.put(task.wait_for_sec(wait_for_sec))
        await self._queue.put(task.wait_for_sec(wait_for_sec))

    def task_done(self) -> None:
        self._queue.task_done()

    async def join(self) -> None:
        await self._queue.join()
        if self._db is not None:
            await self._db.close()
