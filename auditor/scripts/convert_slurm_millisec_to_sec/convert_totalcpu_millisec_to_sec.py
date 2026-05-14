import os
from pathlib import Path
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
TOTAL_CPU = os.getenv("TOTAL_CPU_TIME")


def main():
    conn = None
    try:
        conn = psycopg2.connect(**DB_CONFIG)
        cursor = conn.cursor()

        update_query = """
            UPDATE auditor_accounting
            SET components = (
                SELECT jsonb_agg(
                    CASE
                        WHEN elem->>'name' = %(cpu)s
                        THEN jsonb_set(elem, '{amount}', to_jsonb((elem->>'amount')::bigint / 1000))
                        ELSE elem
                    END
                ) ||
                -- Append the _milli entry
                jsonb_build_array(
                    jsonb_build_object(
                        'name',   %(cpu_milli)s,
                        'amount', (
                            SELECT (e->>'amount')::bigint
                            FROM jsonb_array_elements(components) e
                            WHERE e->>'name' = %(cpu)s
                            LIMIT 1
                        ),
                        'scores', '[]'::jsonb
                    )
                )
                FROM jsonb_array_elements(components) elem
            )
            WHERE
                components @> jsonb_build_array(jsonb_build_object('name', %(cpu)s))
                AND NOT (components @> jsonb_build_array(jsonb_build_object('name', %(cpu_milli)s)));
        """

        cursor.execute(
            update_query,
            {
                "cpu": TOTAL_CPU,
                "cpu_milli": TOTAL_CPU + "_milli",
            },
        )

        updated_rows = cursor.rowcount
        conn.commit()
        print(f"Done! Updated {updated_rows} rows.")

    except Exception as e:
        print(f"Error: {e}")
        if conn:
            conn.rollback()
    finally:
        if cursor:
            cursor.close()
        if conn:
            conn.close()


if __name__ == "__main__":
    main()
