import json
import unittest
from unittest.mock import MagicMock, patch
from urllib.parse import quote

from revert_encodings import decode_record, main


class TestDecodeRecord(unittest.TestCase):
    def test_decode_record_success(self):
        # Test for successful decoding
        record_id = quote("test_record_id/")
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


@patch("main_script.psycopg2.connect")
def test_main(mock_connect):
    # Mock database connection and cursor
    mock_conn = MagicMock()
    mock_cursor = MagicMock()

    # Setup mock connection
    mock_connect.return_value = mock_conn
    mock_conn.cursor.return_value = mock_cursor

    # Mock database rows
    rows = [
        (1, quote("record1/"), {"key1": [quote("value1/")]}),
        (2, quote("record2/"), {"key2": [quote("value2/")]}),
    ]

    mock_cursor.fetchall.side_effect = [rows, []]  # Return rows and then stop

    def mock_decode_record(record_id, meta):
        return record_id.replace("%20", " "), json.dumps(meta)

    with patch("revert_encodings.decode_record", side_effect=mock_decode_record):
        main()

    # Assertions
    mock_cursor.execute.assert_any_call(
        "SELECT id, record_id, meta FROM auditor_accounting ORDER BY id LIMIT 1000 OFFSET 0;"
    )

    for row in rows:
        mock_cursor.execute.assert_any_call(
            """
            UPDATE auditor_accounting
            SET record_id = %s, meta = %s
            WHERE id = %s;
            """,
            (row[1].replace("%20", " "), json.dumps(row[2]), row[0]),
        )

    mock_conn.commit.assert_called()
    mock_cursor.close.assert_called()
    mock_conn.close.assert_called()


if __name__ == "__main__":
    unittest.main()
