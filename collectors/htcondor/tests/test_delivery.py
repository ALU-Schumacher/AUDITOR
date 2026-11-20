import logging
import unittest
from unittest.mock import AsyncMock, Mock

from auditor_htcondor_collector.collector import CondorHistoryCollector


class DeliveryTests(unittest.IsolatedAsyncioTestCase):
    def collector(self):
        collector = CondorHistoryCollector.__new__(CondorHistoryCollector)
        collector.config = {}
        collector.logger = logging.getLogger(__name__)
        collector.get_last_job = Mock(return_value=(1, 0))
        collector.query_htcondor_history = AsyncMock(return_value=[{"ClusterId": 2, "ProcId": 0}])
        collector._generate_record = Mock(return_value=object())
        collector.client = Mock(add=AsyncMock())
        collector.set_last_job = Mock()
        return collector

    async def test_rejected_delivery_does_not_advance_checkpoint(self):
        collector = self.collector()
        collector.client.add.side_effect = RuntimeError("HTTP 500")
        with self.assertRaises(RuntimeError):
            await collector._collect("synthetic-schedd")
        collector.set_last_job.assert_not_called()

    async def test_confirmed_delivery_advances_checkpoint(self):
        collector = self.collector()
        await collector._collect("synthetic-schedd")
        collector.set_last_job.assert_called_once_with("synthetic-schedd", (2, 0))
