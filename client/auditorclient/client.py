from __future__ import annotations  # not necessary in 3.10
import asyncio
import aiohttp
import logging
from aiohttp.client_exceptions import ClientConnectorError
from .task import Task, Instruction
from .queue import Queue
from .db import DB, DBsqlite
from .record import Record
from .errors import RecordExistsError, RecordDoesNotExistError


class AuditorClient:
    _headers = {"content-type": "application/json"}

    def __init__(
        self,
        host: str,
        port: int,
        timeout: int = 10,
        retries: int = 5,
        num_workers: int = 1,
        delay_before_retry: int = 5,
        db: DB = DBsqlite(),
    ) -> AuditorClient:
        self._host = host
        self._port = port
        self._session = None
        self._queue = Queue(db=db)
        self._timeout = aiohttp.ClientTimeout(total=timeout)
        self._retries = retries
        self._num_workers = num_workers
        self._delay_before_retry = delay_before_retry
        self._logger = logging.getLogger("auditorclient.client.AuditorClient")

    async def start(self) -> None:
        self._session = aiohttp.ClientSession(
            headers=self._headers, timeout=self._timeout
        )
        self._logger.info(f"Spawning {self._num_workers} workers")
        self._workers = [
            asyncio.create_task(self._worker(i)) for i in range(self._num_workers)
        ]
        await self._queue.start()

    async def stop(self) -> None:
        logging.info("Stopping client, waiting until queue is empty.")
        await self._queue.join()
        for w in self._workers:
            self._logger.debug(f"Stopping worker {w}")
            w.cancel()
        await self._session.close()

    async def _worker(self, worker_id: int) -> None:
        while True:
            try:
                token = await self._queue.get()
                if token.try_once():
                    record = token.record()
                    if token.instr() == Instruction.ADD:
                        try:
                            await self.add_record(record)
                        except RecordExistsError:
                            self._logger.warning(
                                f"Worker {worker_id}: "
                                + f"Record {record.record_id()} of site {record.site_id()}"
                                + " not sent and not requeued."
                            )
                        except ClientConnectorError:
                            self._logger.warning(
                                f"Worker {worker_id}: "
                                f"Connection refused. Requeuing record {record.record_id()}"
                                + f" of site {record.site_id()} "
                                + f"({self._retries-token.retries()}/{self._retries})."
                            )
                            if token.retries() > 0:
                                await self._queue.put(
                                    token, wait_for_sec=self._delay_before_retry
                                )
                        except Exception as e:
                            self._logger.error(e)
                    elif token.instr() == Instruction.UPDATE:
                        try:
                            await self.update_record(record)
                        except RecordDoesNotExistError:
                            self._logger.warning(
                                f"Worker {worker_id}: "
                                f"Record {record.record_id()} of site {record.site_id()}"
                                + " not sent, requeueing."
                            )
                            if token.retries() > 0:
                                await self._queue.put(
                                    token, wait_for_sec=self._delay_before_retry
                                )
                        except ClientConnectorError:
                            self._logger.warning(
                                f"Worker {worker_id}: "
                                f"Connection refused. Requeuing record {record.record_id()}"
                                + f" of site {record.site_id()} "
                                + f"({self._retries-token.retries()}/{self._retries})."
                            )
                            if token.retries() > 0:
                                await self._queue.put(
                                    token, wait_for_sec=self._delay_before_retry
                                )
                        except Exception as e:
                            self._logger.error(e)
                self._queue.task_done()
            except Exception as e:
                self._logger.warning(f"Worker {worker_id}: Exception: {e}")

    async def add_record_queue(self, record: Record) -> None:
        self._logger.debug(f"Adding ADD record to queue: {record}")
        await self._queue.put(Task(Instruction.ADD, record, retries=self._retries))

    async def update_record_queue(self, record: Record) -> None:
        self._logger.debug(f"Adding UPDATE record to queue: {record}")
        await self._queue.put(Task(Instruction.UPDATE, record, retries=self._retries))

    async def add_record(self, record: Record):
        self._logger.debug(
            f"Adding record {record} to AUDITOR instance running at"
            + f" http://{self._host}:{self._port}"
        )
        async with self._session.post(
            f"http://{self._host}:{self._port}/add",
            data=record.as_json(),
        ) as response:
            if response.status == 409:
                self._logger.warning(
                    f"Record {record.record_id()} of site {record.site_id()} already exists at"
                    + f" http://{self._host}:{self._port}."
                )
                raise RecordExistsError(record.record_id(), record.site_id())
            return response

    async def update_record(self, record: Record) -> str:
        self._logger.debug(
            f"Updating record {record} of AUDITOR instance running at"
            + f" http://{self._host}:{self._port}"
        )
        async with self._session.post(
            f"http://{self._host}:{self._port}/update",
            data=record.as_json(),
        ) as response:
            if response.status == 400:
                self._logger.warning(
                    f"Record {record.record_id()} of site {record.site_id()} cannot be updated "
                    + f"because it does not exist at http://{self._host}:{self._port}."
                )
                raise RecordDoesNotExistError(record.record_id(), record.site_id())
            return response

    async def get(self) -> dict:
        async with self._session.get(
            f"http://{self._host}:{self._port}/get"
        ) as response:
            return await response.json()

    async def get_since(self, timestamp: str) -> dict:
        async with self._session.get(
            f"http://{self._host}:{self._port}/get/since/{timestamp}"
        ) as response:
            return await response.json()
