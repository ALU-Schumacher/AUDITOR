#!/usr/bin/env python3

import requests
import time
import unittest


class TestSingleEntry(unittest.TestCase):
    def setUp(self):
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
        # wait for metrics recomputation
        time.sleep(10)

    def test(self):
        r = requests.get("http://127.0.0.1:8000/metrics")
        self.assertEqual(r.status_code, 200)
        self.assertIn("num_records_database 1\n", r.text)
        self.assertIn('num_records_database_per_group{group_id="group1"} 1\n', r.text)
        self.assertIn('num_records_database_per_site{site="site1"} 1\n', r.text)
        self.assertIn('num_records_database_per_user{user_id="user1"} 1\n', r.text)


if __name__ == "__main__":
    unittest.main()
