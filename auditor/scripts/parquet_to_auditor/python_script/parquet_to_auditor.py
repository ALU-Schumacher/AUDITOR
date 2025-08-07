import json
import os
from datetime import datetime, timezone
from pathlib import Path

import pandas as pd
import psycopg2
import pyarrow.parquet as pq
from dotenv import load_dotenv
from psycopg2.extras import Json, execute_values

load_dotenv(dotenv_path=Path(__file__).parent / ".env", override=False)


DB_CONFIG = {
    "dbname": os.getenv("DB_NAME"),
    "user": os.getenv("DB_USER"),
    "password": os.getenv("DB_PASSWORD"),
    "host": os.getenv("DB_HOST"),
    "port": os.getenv("DB_PORT"),
}


def main():
    parquet_path = os.getenv("PARQUET_PATH")
    for batch_df in read_parquet_in_batches(parquet_path):
        batch_insert(batch_df)


def read_parquet_in_batches(parquet_path, batch_size=300000):
    pf = pq.ParquetFile(parquet_path)

    for rg in range(pf.num_row_groups):
        table = pf.read_row_group(rg)
        df = table.to_pandas()

        for start in range(0, len(df), batch_size):
            yield df.iloc[start : start + batch_size]


def batch_insert(df):
    # Ensure time columns are timezone-aware and in UTC
    df.loc[:, "start_time"] = pd.to_datetime(df["start_time"], utc=True)
    df.loc[:, "stop_time"] = pd.to_datetime(df["stop_time"], utc=True)

    # Connect to PostgreSQL
    conn = psycopg2.connect(**DB_CONFIG)

    cursor = conn.cursor()

    insert_query = """
        INSERT INTO auditor_accounting (
            record_id,
            meta,
            components,
            start_time,
            stop_time,
            runtime,
            updated_at
        ) VALUES %s
    """

    records = []
    for _, row in df.iterrows():
        try:
            meta = (
                json.loads(row["meta"]) if isinstance(row["meta"], str) else row["meta"]
            )
            components = (
                json.loads(row["components"])
                if isinstance(row["components"], str)
                else row["components"]
            )

            records.append(
                (
                    row["record_id"],
                    Json(meta),
                    Json(components),
                    row["start_time"].to_pydatetime(),
                    row["stop_time"].to_pydatetime(),
                    row["runtime"],
                    datetime.now(timezone.utc),
                )
            )
        except Exception as e:
            print(f"Skipping row due to error: {e}")
            continue

    if records:
        try:
            execute_values(cursor, insert_query, records)
            conn.commit()
            print(f"Inserted {len(records)} records.")
        except Exception as e:
            print(f"Error inserting batch: {e}")
            conn.rollback()

    cursor.close()
    conn.close()


if __name__ == "__main__":
    main()
