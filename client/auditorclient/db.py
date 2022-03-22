from __future__ import annotations  # not necessary in 3.10
from abc import ABC, abstractmethod
import aiosqlite
import os.path
import logging
from dateutil import parser
from .task import Task, Instruction
from .record import Record, Components


class DB(ABC):
    @abstractmethod
    def start(self):
        pass

    @abstractmethod
    def close(self):
        pass

    @abstractmethod
    def get_all(self) -> [Task]:
        pass

    @abstractmethod
    def put(self, task: Task):
        pass

    @abstractmethod
    def delete(self, task: Task):
        pass


class DBsqlite(DB):
    def __init__(self, filename: str = "database.db"):
        self._filename = filename if filename else "database.db"
        self._logger = logging.getLogger("auditorclient.dbsqlite.DBsqlite")

    async def start(self):
        self._logger.debug(f"Starting DBsqlite database ({self._filename})")
        if not os.path.isfile(self._filename):
            self._logger.debug(
                f"DBsqlite: database file {self._filename} not"
                + " found, initializing empty database."
            )
            self._db = await aiosqlite.connect(self._filename)
            cur = await self._db.execute(
                """
                    CREATE TABLE auditorclient
                    (
                        record_id VARCHAR(50) NOT NULL,
                        site_id VARCHAR(50) NOT NULL,
                        instruction INT NOT NULL,
                        record TEXT NOT NULL,
                        retries INT NOT NULL,
                        schedule_after TIMESTAMP,
                        PRIMARY KEY (record_id, site_id, instruction)
                    );
                """
            )
            await cur.close()
        else:
            self._db = await aiosqlite.connect(self._filename)

    async def close(self):
        self._logger.debug("Closing database connection")
        await self._db.close()

    async def delete(self, task: Task):
        self._logger.debug(f"DBsqlite: Deleting task from database: {task}")
        instr = task.instr()
        record = task.record()
        cur = await self._db.execute(
            f"""
                DELETE
                FROM auditorclient
                WHERE record_id='{record.record_id()}'
                AND site_id='{record.site_id()}'
                AND instruction={instr.value}
            """
        )
        await self._db.commit()
        await cur.close()

    async def get_all(self) -> [Task]:
        self._logger.debug("DBsqlite: Retrieving entire database")
        cur = await self._db.execute(
            """
                SELECT * FROM auditorclient
            """
        )
        rows = await cur.fetchall()
        return [
            Task(
                Instruction(row[2]),
                Record(json_str=row[3]),
                row[4],
            ).with_schedule_after(parser.parse(row[5]) if row[5] != "None" else None)
            for row in rows
        ]

    async def put(self, task: Task):
        self._logger.debug(f"DBsqlite: Adding task to database: {task}")
        instr = task.instr()
        record = task.record()
        retries = task.retries()
        schedule_after = task.schedule_after()
        cur = await self._db.execute(
            f"""
                INSERT INTO auditorclient VALUES
                (
                    '{record.record_id()}',
                    '{record.site_id()}',
                    {instr.value},
                    '{record.as_json()}',
                    {retries},
                    '{schedule_after}'
                )
            """
        )
        await self._db.commit()
        await cur.close()


class MockDB(DB):
    def __init__(self, empty_db=False):
        self._start_count = 0
        self._close_count = 0
        self._get_all_count = 0
        self._put_count = 0
        self._delete_count = 0
        self._last_called = None
        if empty_db:
            self._tasks = []
        else:
            self._tasks = [
                Task(
                    Instruction.ADD,
                    Record(
                        "from_db",
                        "site",
                        "user",
                        "group",
                        Components().add_component("comp1", 1, 2.0),
                    ),
                    5,
                )
            ]

    async def start(self):
        self._last_called = "start"
        self._start_count += 1

    async def close(self):
        self._last_called = "close"
        self._close_count += 1

    async def get_all(self) -> [Task]:
        self._last_called = "get_all"
        self._get_all_count += 1
        return self._tasks

    async def put(self, task: Task):
        self._last_called = "put"
        self._put_count += 1
        self._tasks.append(task)

    async def delete(self, _task: Task):
        self._last_called = "delete"
        self._delete_count += 1
        self._tasks.pop()

    def get_counts(self):
        return [
            self._start_count,
            self._close_count,
            self._get_all_count,
            self._put_count,
            self._delete_count,
        ]

    def get_last_called(self):
        return self._last_called
