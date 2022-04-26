from unittest import IsolatedAsyncioTestCase, TestCase, mock
from auditorclient.db import DB, DBsqlite
from auditorclient.record import Record, Components, Scores
from auditorclient.task import Task, Instruction

import aiosqlite
import asyncio
import os
from os.path import isfile


class TestDB(TestCase):
    def test_DB(self):
        with self.assertRaises(TypeError):
            db = DB()


class TestDBsqlite(IsolatedAsyncioTestCase):
    def setUp(self):
        self.test_db = os.path.join(
            os.path.dirname(os.path.realpath(__file__)), "test.db"
        )
        try:
            os.remove(self.test_db)
        except FileNotFoundError:
            pass
        self.sql_create_cmd = " ".join(
            """CREATE TABLE auditorclient
               (
                   record_id VARCHAR(50) NOT NULL,
                   site_id VARCHAR(50) NOT NULL,
                   instruction INT NOT NULL,
                   record TEXT NOT NULL,
                   retries INT NOT NULL,
                   schedule_after TIMESTAMP,
                   PRIMARY KEY (record_id, site_id, instruction)
               )""".split()
        )

    #  @mock.patch("auditorclient.db.os.path.isfile", wraps=isfile)
    #  @mock.patch("auditorclient.db.aiosqlite", wraps=aiosqlite)
    async def test_DBsqlite(self):  # , mock_isfile):
        # test default
        db = DBsqlite()
        self.assertEqual(db._filename, "database.db")

        # set test db
        db = DBsqlite(filename=self.test_db)
        self.assertEqual(db._filename, self.test_db)

        await db.start()

        async with aiosqlite.connect(self.test_db) as con:
            async with con.execute(
                "SELECT sql FROM sqlite_master WHERE name='auditorclient';"
            ) as cursor:
                row = await cursor.fetchone()
                self.assertEqual(" ".join(row[0].split()), self.sql_create_cmd)

        record = Record(
            "record",
            "site",
            "user",
            "group",
            Components().add_component("comp1", 1, Scores().add_score("score1", 2.0)),
        )
        retries = 5
        task = Task(Instruction.ADD, record, retries)

        await db.put(task)

        async with aiosqlite.connect(self.test_db) as con:
            async with con.execute("SELECT * FROM auditorclient;") as cursor:
                row = await cursor.fetchone()
                self.assertEqual(Record(json_str=row[3]), record)

        tasks = await db.get_all()
        self.assertEqual(tasks[0], task)

        await db.close()
