#!/usr/bin/env python3

import unittest

import requests


class TestSlurmEncodingReversal(unittest.TestCase):
    def test_check_db_records(self):
        record_1 = {
            "record_id": "record1",
            "meta": {
                "site_id": ["site/1"],
                "user_id": ["user/1"],
                "group_id": ["group/1"],
            },
            "components": [
                {
                    "name": "NumCPUs",
                    "amount": 31,
                    "scores": [{"name": "HEPSPEC", "value": 1.2}],
                }
            ],
            "start_time": "2022-06-27T15:00:00Z",
            "stop_time": "2022-06-27T15:01:00Z",
            "runtime": 6,
        }

        record_2 = {
            "record_id": "record2",
            "meta": {
                "site_id": ["site/2"],
                "user_id": ["user/2"],
                "group_id": ["group/2"],
            },
            "components": [
                {
                    "name": "NumCPUs",
                    "amount": 31,
                    "scores": [{"name": "HEPSPEC", "value": 1.2}],
                }
            ],
            "start_time": "2022-06-27T15:00:00Z",
            "stop_time": "2022-06-27T15:01:00Z",
            "runtime": 60,
        }

        response = requests.get("http://localhost:8000/records")

        if response.status_code != 200:
            print(f"Failed to get record: {response.status_code}, {response.text}")
        else:
            print("Successfully retrieved records ", len(response.json()))

        records = sorted(response.json(), key=lambda x: x.get("record_id"))

        self.assertEqual(records[0].get("record_id"), record_1.get("record_id"))
        self.assertEqual(records[0].get("meta"), record_1.get("meta"))

        self.assertEqual(records[1].get("record_id"), record_2.get("record_id"))
        self.assertEqual(records[1].get("meta"), record_2.get("meta"))


# Call the function
if __name__ == "__main__":
    unittest.main()
