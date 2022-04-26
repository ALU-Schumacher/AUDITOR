from unittest import IsolatedAsyncioTestCase, TestCase, mock
import auditorclient
from auditorclient.db import MockDB
from auditorclient.record import Record, Components, Scores
from auditorclient.task import Task, Instruction
from auditorclient.client import AuditorClient
from auditorclient.errors import RecordExistsError, RecordDoesNotExistError
from aioresponses import aioresponses
from aiohttp.client_exceptions import ClientConnectorError

import json
import aiohttp
import aiosqlite
import asyncio
import os
from os.path import isfile


class TestAuditorClient(IsolatedAsyncioTestCase):
    def setUp(self):

        self.get_response = [
            {
                "record_id": "record1",
                "site_id": "site",
                "user_id": "user",
                "group_id": "grop",
                "components": [
                    {
                        "name": "CPU",
                        "amount": 1,
                        "scores": [{"name": "score1", "factor": 1.3}],
                    }
                ],
                "start_time": "2019-11-28T12:45:59.324310Z",
                "stop_time": "2020-11-29T12:45:59.324310Z",
                "runtime": 31708800,
                "updated_at": "2021-06-09T08:45:15.301872Z",
            },
            {
                "record_id": "record2",
                "site_id": "site",
                "user_id": "user",
                "group_id": "group",
                "components": [
                    {
                        "name": "CPU",
                        "amount": 2,
                        "scores": [{"name": "score1", "factor": 1.3}],
                    }
                ],
                "start_time": "2019-11-28T12:45:59.324310Z",
                "stop_time": "2020-11-29T12:45:59.324310Z",
                "runtime": 31708800,
                "updated_at": "2021-06-09T08:45:58.041086Z",
            },
        ]

    @aioresponses()
    async def test_AuditorClient(self, mocked):
        # test default settings
        client = AuditorClient("localhost", 8080)
        self.assertEqual(client._timeout, aiohttp.ClientTimeout(total=10))
        self.assertEqual(client._retries, 5)
        self.assertEqual(client._num_workers, 1)
        self.assertEqual(client._delay_before_retry, 5)

        # test manually set values
        client = AuditorClient(
            "localhost",
            8080,
            timeout=2,
            retries=2,
            num_workers=2,
            delay_before_retry=2,
            db=MockDB(),
        )
        self.assertEqual(client._timeout, aiohttp.ClientTimeout(total=2))
        self.assertEqual(client._retries, 2)
        self.assertEqual(client._num_workers, 2)
        self.assertEqual(client._delay_before_retry, 2)

        await client.start()

        self.assertEqual(len(client._workers), 2)

        mocked.post("http://localhost:8080/add", status=200, body="test")

        record = Record(
            "from_test",
            "site",
            "user",
            "group",
            Components().add_component("comp1", 1, Scores().add_score("score1", 2.0)),
        )

        response = await client.add_record(record)
        self.assertEqual(response.status, 200)

        with self.assertRaises(RecordExistsError):
            mocked.post("http://localhost:8080/add", status=409)
            await client.add_record(record)

        with self.assertRaises(RecordDoesNotExistError):
            mocked.post("http://localhost:8080/update", status=400)
            await client.update_record(record)

        mocked.post("http://localhost:8080/update", status=200, body="test")
        response = await client.update_record(record)
        self.assertEqual(response.status, 200)

        mocked.get(
            "http://localhost:8080/get", status=200, body=json.dumps(self.get_response)
        )
        response = await client.get()
        self.assertEqual(response, self.get_response)

        mocked.get(
            "http://localhost:8080/get/started/since/2021-06-08T00:00:00.000000Z",
            status=200,
            body=json.dumps(self.get_response),
        )
        response = await client.get_started_since("2021-06-08T00:00:00.000000Z")
        self.assertEqual(response, self.get_response)

        mocked.get(
            "http://localhost:8080/get/stopped/since/2021-06-08T00:00:00.000000Z",
            status=200,
            body=json.dumps(self.get_response),
        )
        response = await client.get_stopped_since("2021-06-08T00:00:00.000000Z")
        self.assertEqual(response, self.get_response)

        mocked.post("http://localhost:8080/add", status=409, body="test")
        await client.add_record_queue(record)

        await client.stop()

    @aioresponses()
    async def test_AuditorClient_workers(self, mocked):
        client = AuditorClient(
            "localhost",
            8080,
            timeout=2,
            retries=2,
            num_workers=0,
            delay_before_retry=2,
            db=MockDB(empty_db=True),
        )

        await client.start()

        record = Record(
            "from_test",
            "site",
            "user",
            "group",
            Components().add_component("comp1", 1, Scores().add_score("score1", 2.0)),
        )

        mocked.post("http://localhost:8080/add", status=200, body="test")
        await client.add_record_queue(record)

        self.assertEqual(client._queue._queue.qsize(), 1)

        w = asyncio.create_task(client._worker(0))

        # potentially problematic
        await asyncio.sleep(0.2)
        self.assertEqual(client._queue._queue.qsize(), 0)

        with self.assertLogs(logger=None, level="WARNING") as cm:
            mocked.post("http://localhost:8080/add", status=409, body="test")
            await client.add_record_queue(record)
            await asyncio.sleep(0.2)
            self.assertEqual(
                cm.output,
                [
                    "WARNING:auditorclient.client.AuditorClient:Record from_test of"
                    + " site site already exists at http://localhost:8080.",
                    "WARNING:auditorclient.client.AuditorClient:Worker 0: Record"
                    + " from_test of site site not sent and not requeued.",
                ],
            )

        with self.assertLogs(logger=None, level="WARNING") as cm:
            # need this twice because worker will execute two POST requests
            mocked.post(
                "http://localhost:8080/add",
                exception=ClientConnectorError(None, OSError()),
            )
            mocked.post(
                "http://localhost:8080/add",
                exception=ClientConnectorError(None, OSError()),
            )
            await client.add_record_queue(record)
            # need to wait quite long here...
            await asyncio.sleep(4.0)
            self.assertEqual(
                cm.output,
                [
                    "WARNING:auditorclient.client.AuditorClient:Worker 0:"
                    + " Connection refused. Requeuing record from_test of site site"
                    + " (1/2).",
                    "WARNING:auditorclient.client.AuditorClient:Worker 0:"
                    + " Connection refused. Requeuing record from_test of site site"
                    + " (2/2).",
                ],
            )

        with self.assertLogs(logger=None, level="WARNING") as cm:
            mocked.post("http://localhost:8080/update", status=400, body="test")
            mocked.post("http://localhost:8080/update", status=400, body="test")
            await client.update_record_queue(record)
            await asyncio.sleep(4.5)
            self.assertEqual(
                cm.output,
                [
                    "WARNING:auditorclient.client.AuditorClient:Record from_test of"
                    + " site site cannot be updated because it does not exist at"
                    + " http://localhost:8080.",
                    "WARNING:auditorclient.client.AuditorClient:Worker 0: Record"
                    + " from_test of site site not sent, requeueing.",
                    "WARNING:auditorclient.client.AuditorClient:Record from_test of"
                    + " site site cannot be updated because it does not exist at"
                    + " http://localhost:8080.",
                    "WARNING:auditorclient.client.AuditorClient:Worker 0: Record"
                    + " from_test of site site not sent, requeueing.",
                ],
            )

        with self.assertLogs(logger=None, level="WARNING") as cm:
            # need this twice because worker will execute two POST requests
            mocked.post(
                "http://localhost:8080/update",
                exception=ClientConnectorError(None, OSError()),
            )
            mocked.post(
                "http://localhost:8080/update",
                exception=ClientConnectorError(None, OSError()),
            )
            await client.update_record_queue(record)
            # need to wait quite long here...
            await asyncio.sleep(4.0)
            self.assertEqual(
                cm.output,
                [
                    "WARNING:auditorclient.client.AuditorClient:Worker 0:"
                    + " Connection refused. Requeuing record from_test of site site"
                    + " (1/2).",
                    "WARNING:auditorclient.client.AuditorClient:Worker 0:"
                    + " Connection refused. Requeuing record from_test of site site"
                    + " (2/2).",
                ],
            )

        w.cancel()

        await client.stop()
