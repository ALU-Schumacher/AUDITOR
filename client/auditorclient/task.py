from __future__ import annotations  # not necessary in 3.10
from enum import IntEnum
import datetime
from .record import Record


# Needs to be IntEnum because it needs to be ordered for the PriorityQueue where it is
# used in.
class Instruction(IntEnum):
    ADD = 1
    UPDATE = 2


class Task:
    def __init__(self, instr: Instruction, record: Record, retries: int):
        self._instr = instr
        self._record = record
        self._retries = retries
        self._schedule_after = None

    def __lt__(self, other: Task) -> bool:
        return self._instr < other._instr

    # This is used only for tests; however, this might be problematic because it is
    # inconsistent with the __lt__ implementation. Check here if there are problems
    # with sorting in the PriorityQueue.
    def __eq__(self, other: Task) -> bool:
        return (
            self._instr == other._instr
            and self._record == other._record
            and self._retries == other._retries
            and self._schedule_after == other._schedule_after
        )

    def instr(self) -> Instruction:
        return self._instr

    def record(self) -> Record:
        return self._record

    def retries(self) -> int:
        return self._retries

    def with_schedule_after(self, schedule_after: datetime.datetime) -> Task:
        self._schedule_after = schedule_after
        return self

    def wait_for_sec(self, time: int) -> Task:
        if time is not None:
            self._schedule_after = datetime.datetime.now() + datetime.timedelta(
                seconds=time
            )
        else:
            self._schedule_after = None
        return self

    def schedule_after(self):
        return self._schedule_after

    def try_once(self) -> bool:
        self._retries -= 1
        return self._retries >= 0
