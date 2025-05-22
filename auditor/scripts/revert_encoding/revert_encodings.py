import json
import os
from json.decoder import JSONDecodeError
from pathlib import Path
from urllib.parse import unquote

import psycopg2
from dotenv import load_dotenv

load_dotenv(dotenv_path=Path(__file__).parent / ".env")


DB_CONFIG = {
    "dbname": os.getenv("DB_NAME"),
    "user": os.getenv("DB_USER"),
    "password": os.getenv("DB_PASSWORD"),
    "host": os.getenv("DB_HOST"),
    "port": os.getenv("DB_PORT"),
}


def decode_record(record_id, meta):
    """
    Decode the record_id and meta values.
    """
    decoded_record_id = unquote(record_id)

    try:
        decoded_meta = {
            key: [unquote(value) for value in values] for key, values in meta.items()
        }
    except Exception as e:
        raise JSONDecodeError(f"Error decoding meta: {e}", str(meta), 0)
    return decoded_record_id, json.dumps(decoded_meta)


def main():
    BATCH_SIZE = 1000
    offset = 0
    conn = None
    cursor = None

    try:
        conn = psycopg2.connect(**DB_CONFIG)
        while True:
            cursor = conn.cursor()

            fetch_query = f"SELECT id, record_id, meta FROM auditor_accounting ORDER BY id LIMIT {BATCH_SIZE} OFFSET {offset};"
            cursor.execute(fetch_query)
            rows = cursor.fetchall()

            if not rows:
                break

            for row in rows:
                record_id, meta = row[1], row[2]
                decoded_record_id, decoded_meta = decode_record(record_id, meta)

                update_query = """
                    UPDATE auditor_accounting
                    SET record_id = %s, meta = %s
                    WHERE id = %s;
                """
                cursor.execute(update_query, (decoded_record_id, decoded_meta, row[0]))

            conn.commit()
            offset += BATCH_SIZE

        print("Database updated successfully!")

    except Exception as e:
        print(f"Error: {e}")
        if conn:
            conn.rollback()  # Rollback on error

    finally:
        if cursor:
            cursor.close()
        if conn:
            conn.close()


if __name__ == "__main__":
    main()
