import asyncio
from auditorclient.db import MockDB
from auditorclient.queue import Queue
from auditorclient.task import Task, Instruction
from auditorclient.record import Record, Components
from unittest import IsolatedAsyncioTestCase, TestCase, mock


class TestQueue(IsolatedAsyncioTestCase):
    def setUp(self):
        pass

    async def test_queue(self):
        mock_db = MockDB()
        queue = Queue(db=mock_db)

        await queue.start()
        self.assertEqual(mock_db.get_counts(), [1, 0, 1, 0, 0])
        self.assertEqual(mock_db.get_last_called(), "get_all")
        task = await queue.get()
        self.assertEqual(task.record().record_id(), "from_db")
        self.assertEqual(mock_db.get_counts(), [1, 0, 1, 0, 1])
        queue.task_done()

        task = Task(
            Instruction.ADD,
            Record(
                "from_test",
                "site",
                "user",
                "group",
                Components().add_component("comp1", 1, 2.0),
            ),
            5,
        )
        await queue.put(task)
        self.assertEqual(mock_db.get_counts(), [1, 0, 1, 1, 1])

        task = await queue.get()
        self.assertEqual(task.record().record_id(), "from_test")
        self.assertEqual(mock_db.get_counts(), [1, 0, 1, 1, 2])
        queue.task_done()

        await queue.join()
        self.assertEqual(mock_db.get_counts(), [1, 1, 1, 1, 2])
