#!/usr/bin/env python3

import unittest

import requests


class TestEmptyDB(unittest.TestCase):
    def test(self):
        r = requests.get("http://127.0.0.1:8000/metrics")
        self.assertEqual(r.status_code, 200)
        self.assertIn("num_records_database 0\n", r.text)


if __name__ == "__main__":
    unittest.main()
