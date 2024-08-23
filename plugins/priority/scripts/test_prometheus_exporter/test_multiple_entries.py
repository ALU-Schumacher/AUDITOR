import time
import unittest
from datetime import datetime, timedelta

import requests


class TestMultipleEntries(unittest.TestCase):
    def setUp(self):
        current_time_utc = datetime.utcnow()
        two_days_ago = current_time_utc - timedelta(days=2)

        stop_time = current_time_utc.strftime("%Y-%m-%dT%H:%M:%SZ")
        start_time = two_days_ago.strftime("%Y-%m-%dT%H:%M:%SZ")

        record1_data = {
            "record_id": "record-1",
            "meta": {
                "site_id": ["site1"],
                "group_id": ["atlsch"],
                "user_id": ["user1"],
            },
            "components": [
                {
                    "name": "NumCPUs",
                    "amount": 8,
                    "scores": [{"name": "HEPSPEC", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": start_time,
            "stop_time": stop_time,
        }
        requests.post("http://127.0.0.1:8000/record", json=record1_data)

        record11_data = {
            "record_id": "record-11",
            "meta": {
                "site_id": ["site1"],
                "group_id": ["atlsch"],
                "user_id": ["user1"],
            },
            "components": [
                {
                    "name": "NumCPUs",
                    "amount": 8,
                    "scores": [{"name": "HEPSPEC", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": start_time,
            "stop_time": stop_time,
        }
        requests.post("http://127.0.0.1:8000/record", json=record11_data)

        # add second record
        record2_data = {
            "record_id": "record-2",
            "meta": {
                "site_id": ["site1"],
                "group_id": ["atljak"],
                "user_id": ["user1"],
            },
            "components": [
                {
                    "name": "NumCPUs",
                    "amount": 15,
                    "scores": [{"name": "HEPSPEC", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": start_time,
            "stop_time": stop_time,
        }
        requests.post("http://127.0.0.1:8000/record", json=record2_data)

        record22_data = {
            "record_id": "record-22",
            "meta": {
                "site_id": ["site1"],
                "group_id": ["atljak"],
                "user_id": ["user1"],
            },
            "components": [
                {
                    "name": "NumCPUs",
                    "amount": 15,
                    "scores": [{"name": "HEPSPEC", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": start_time,
            "stop_time": stop_time,
        }
        requests.post("http://127.0.0.1:8000/record", json=record22_data)

        record3_data = {
            "record_id": "record-3",
            "meta": {
                "site_id": ["site1"],
                "group_id": ["atlher"],
                "user_id": ["user2"],
            },
            "components": [
                {
                    "name": "NumCPUs",
                    "amount": 8,
                    "scores": [{"name": "hepspec23", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": start_time,
            "stop_time": stop_time,
        }
        requests.post("http://127.0.0.1:8000/record", json=record3_data)

        record33_data = {
            "record_id": "record-33",
            "meta": {
                "site_id": ["site1"],
                "group_id": ["atlher"],
                "user_id": ["user2"],
            },
            "components": [
                {
                    "name": "NumCPUs",
                    "amount": 11,
                    "scores": [{"name": "hepspec23", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": start_time,
            "stop_time": stop_time,
        }
        requests.post("http://127.0.0.1:8000/record", json=record33_data)

        record4_data = {
            "record_id": "record-4",
            "meta": {
                "site_id": ["site2"],
                "group_id": ["atlhei"],
                "user_id": ["user2"],
            },
            "components": [
                {
                    "name": "NumCPUs",
                    "amount": 8,
                    "scores": [{"name": "hepspec23", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": start_time,
            "stop_time": stop_time,
        }
        requests.post("http://127.0.0.1:8000/record", json=record4_data)

        record44_data = {
            "record_id": "record-44",
            "meta": {
                "site_id": ["site2"],
                "group_id": ["atlhei"],
                "user_id": ["user2"],
            },
            "components": [
                {
                    "name": "NumCPUs",
                    "amount": 14,
                    "scores": [{"name": "hepspec23", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": start_time,
            "stop_time": stop_time,
        }
        requests.post("http://127.0.0.1:8000/record", json=record44_data)

        # wait for metrics recomputation
        time.sleep(10)

    def test(self):
        priority_plugin_prometheus_endpoint = requests.get(
            "http://localhost:9000/metrics"
        )

        self.assertEqual(priority_plugin_prometheus_endpoint.status_code, 200)

        self.assertIn(
            "# TYPE priority gauge\n", priority_plugin_prometheus_endpoint.text
        )
        self.assertIn(
            'priority{group="atlhei"} 2870\n',
            priority_plugin_prometheus_endpoint.text,
        )
        self.assertIn(
            'priority{group="atlher"} 2479\n',
            priority_plugin_prometheus_endpoint.text,
        )
        self.assertIn(
            'priority{group="atljak"} 39123\n',
            priority_plugin_prometheus_endpoint.text,
        )
        self.assertIn(
            'priority{group="atlsch"} 20866\n',
            priority_plugin_prometheus_endpoint.text,
        )

        self.assertIn(
            "# TYPE resource_usage gauge\n", priority_plugin_prometheus_endpoint.text
        )
        self.assertIn(
            'resource_usage{group="atlhei"} 3801600\n',
            priority_plugin_prometheus_endpoint.text,
        )
        self.assertIn(
            'resource_usage{group="atlher"} 3283200\n',
            priority_plugin_prometheus_endpoint.text,
        )
        self.assertIn(
            'resource_usage{group="atljak"} 51840000\n',
            priority_plugin_prometheus_endpoint.text,
        )
        self.assertIn(
            'resource_usage{group="atlsch"} 27648000\n',
            priority_plugin_prometheus_endpoint.text,
        )


if __name__ == "__main__":
    unittest.main()
