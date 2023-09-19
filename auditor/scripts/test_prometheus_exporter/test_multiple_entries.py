#!/usr/bin/env python3

import requests
import time
import unittest


class TestMultipleEntries(unittest.TestCase):
    def setUp(self):
        # add first record
        record1_data = {
            "record_id": "record-1",
            "meta": {
                "site_id": ["site1"],
                "group_id": ["group1"],
                "user_id": ["user1"],
            },
            "components": [
                {
                    "name": "cpu",
                    "amount": 8,
                    "scores": [{"name": "hepspec23", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": "2023-09-18T15:33:00+00:00",
        }
        requests.post("http://127.0.0.1:8000/add", json=record1_data)

        # add second record
        record2_data = {
            "record_id": "record-2",
            "meta": {
                "site_id": ["site1"],
                "group_id": ["group1"],
                "user_id": ["user1"],
            },
            "components": [
                {
                    "name": "cpu",
                    "amount": 8,
                    "scores": [{"name": "hepspec23", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": "2023-09-18T15:33:00+00:00",
        }
        requests.post("http://127.0.0.1:8000/add", json=record2_data)

        record3_data = {
            "record_id": "record-3",
            "meta": {
                "site_id": ["site1"],
                "group_id": ["group2"],
                "user_id": ["user2"],
            },
            "components": [
                {
                    "name": "cpu",
                    "amount": 8,
                    "scores": [{"name": "hepspec23", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": "2023-09-18T15:33:00+00:00",
        }
        requests.post("http://127.0.0.1:8000/add", json=record3_data)

        record4_data = {
            "record_id": "record-4",
            "meta": {
                "site_id": ["site2"],
                "group_id": ["group1"],
                "user_id": ["user2"],
            },
            "components": [
                {
                    "name": "cpu",
                    "amount": 8,
                    "scores": [{"name": "hepspec23", "value": 10}],
                },
                {"name": "mem", "amount": 2048, "scores": []},
            ],
            "start_time": "2023-09-18T15:33:00+00:00",
        }
        requests.post("http://127.0.0.1:8000/add", json=record4_data)

        # wait for metrics recomputation
        time.sleep(10)

    def test(self):
        r = requests.get("http://127.0.0.1:8000/metrics")
        self.assertEqual(r.status_code, 200)
        self.assertIn("num_records_database 4\n", r.text)
        self.assertIn('num_records_database_per_group{group_id="group1"} 3\n', r.text)
        self.assertIn('num_records_database_per_group{group_id="group2"} 1\n', r.text)
        self.assertIn('num_records_database_per_site{site="site1"} 3\n', r.text)
        self.assertIn('num_records_database_per_site{site="site2"} 1\n', r.text)
        self.assertIn('num_records_database_per_user{user_id="user1"} 2\n', r.text)
        self.assertIn('num_records_database_per_user{user_id="user2"} 2\n', r.text)


if __name__ == "__main__":
    unittest.main()
