from auditorclient.task import Instruction, Task
from auditorclient.record import Record, Components, Scores

from unittest import TestCase
from unittest.mock import patch
from unittest import mock

#  from datetime import datetime, timedelta
import datetime


class TestInstruction(TestCase):
    def test_instruction(self):
        self.assertEqual(Instruction.ADD, 1)
        self.assertEqual(Instruction.UPDATE, 2)


class TestTask(TestCase):
    @mock.patch("auditorclient.task.datetime", wraps=datetime)
    def test_task(self, mock_datetime):
        mock_datetime.datetime.now.return_value = datetime.datetime(
            1992, 11, 3, 0, 0, 0
        )
        record = Record(
            "record",
            "site",
            "user",
            "group",
            Components().add_component("comp1", 1, Scores().add_score("score1", 2.0)),
        )
        retries = 5
        task1 = Task(Instruction.ADD, record, retries)
        task2 = Task(Instruction.UPDATE, record, retries)
        self.assertTrue(task1 < task2)
        self.assertFalse(task1 > task2)

        self.assertEqual(task1.instr(), Instruction.ADD)
        self.assertEqual(task1.record(), record)
        self.assertEqual(task1.retries(), retries)

        task1.with_schedule_after(mock_datetime.datetime.now())
        self.assertEqual(task1.schedule_after(), mock_datetime.datetime.now())

        task1.wait_for_sec(5)
        self.assertEqual(
            task1.schedule_after(),
            mock_datetime.datetime.now() + datetime.timedelta(seconds=5),
        )

        self.assertEqual(task1.try_once(), True)
        self.assertEqual(task1.retries(), retries - 1)
        for _i in range(retries - 1):
            task1.try_once()
        self.assertEqual(task1.try_once(), False)
