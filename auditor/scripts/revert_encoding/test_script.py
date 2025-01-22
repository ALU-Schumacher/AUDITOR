import json
import unittest
from unittest.mock import MagicMock, patch
from urllib.parse import quote

from revert_encodings import decode_record, main


class TestDecodeRecord(unittest.TestCase):
    def test_decode_record_success(self):
        # Test for successful decoding
        record_id = quote("test_record_id/", safe="")
        meta = {
            "key1": [quote("value1*"), quote("value2%")],
            "key2": [quote("value3!")],
        }

        expected_record_id = "test_record_id/"
        expected_meta = {"key1": ["value1*", "value2%"], "key2": ["value3!"]}

        decoded_record_id, decoded_meta = decode_record(record_id, meta)

        self.assertEqual(decoded_record_id, expected_record_id)
        self.assertEqual(json.loads(decoded_meta), expected_meta)

    def test_decode_record_failure(self):
        # Test for failure in decoding meta
        record_id = quote("test_record_id")
        meta = "invalid_meta_format"  # Invalid meta format

        with self.assertRaises(Exception) as context:
            decode_record(record_id, meta)

        self.assertIn("Error decoding meta", str(context.exception))


class TestDatabaseUpdate(unittest.TestCase):
    def setUp(self):
        """Set up test cases"""
        self.fetch_query = "SELECT id, record_id, meta FROM auditor_accounting ORDER BY id LIMIT 1000 OFFSET 0;"
        self.update_query = """
                    UPDATE auditor_accounting
                    SET record_id = %s, meta = %s
                    WHERE id = %s;
                """

        # Sample encoded data
        self.sample_id = 1
        self.encoded_record_id = quote("test/record/1", safe="")
        self.encoded_meta = {"key1": [quote("value1")]}

        # Expected decoded data
        self.expected_record_id = "test/record/1"
        self.expected_meta = json.dumps({"key1": ["value1"]})

    @patch("psycopg2.connect")
    def test_main_success(self, mock_connect):
        """Test successful execution of main function"""
        # Set up mock connection and cursor
        mock_cursor = MagicMock()
        mock_conn = MagicMock()
        mock_connect.return_value = mock_conn
        mock_conn.cursor.return_value = mock_cursor

        # Mock the database responses
        mock_cursor.fetchall.side_effect = [
            [
                (self.sample_id, self.encoded_record_id, self.encoded_meta)
            ],  # First batch
            [],  # Empty result to end the loop
        ]

        # Run the main function
        main()

        # Verify the correct SQL queries were executed
        mock_cursor.execute.assert_any_call(self.fetch_query)
        mock_cursor.execute.assert_any_call(
            self.update_query,
            (self.expected_record_id, self.expected_meta, self.sample_id),
        )

        # Verify proper cleanup
        mock_conn.commit.assert_called_once()
        mock_cursor.close.assert_called_once()
        mock_conn.close.assert_called_once()

    @patch("psycopg2.connect")
    def test_main_database_error(self, mock_connect):
        """Test database error handling"""
        # Configure the mock to raise an exception
        mock_connect.side_effect = Exception("Database connection failed")

        # Run the main function - should handle error gracefully
        main()

        # Verify the connection attempt was made
        mock_connect.assert_called_once()


if __name__ == "__main__":
    unittest.main()
