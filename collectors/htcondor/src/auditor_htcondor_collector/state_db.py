import sqlite3
from contextlib import contextmanager
from typing import Generator, Optional, Tuple


class StateDB(object):
    """
    A simple wrapper for an sqlite database for persistent storage of the last
    job id for each scheduler name/record prefix combination.
    The path to the database is set in the config file with the `state_db` key.
    If the database does not exist, it will be created.
    """

    def __init__(self, db_path: str):
        self._db_path = db_path
        self._conn = sqlite3.connect(self._db_path)
        self._cursor = self._conn.cursor()
        self._cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS last_jobs (
                schedd TEXT,
                prefix TEXT,
                cluster INT,
                proc INT,
                PRIMARY KEY (schedd, prefix)
            )
        """
        )
        self._conn.commit()
        self._cursor.close()
        self._conn.close()

    def get(self, schedd: str, prefix: str) -> Optional[Tuple[int, int]]:
        self._cursor.execute(
            "SELECT cluster, proc FROM last_jobs WHERE schedd = ? AND prefix = ?",
            (schedd, prefix),
        )
        row = self._cursor.fetchone()
        if row:
            return row
        return None

    def set(self, schedd: str, prefix: str, cluster: int, proc: int) -> None:
        self._cursor.execute(
            "INSERT OR REPLACE INTO last_jobs VALUES (?, ?, ?, ?)",
            (schedd, prefix, cluster, proc),
        )
        self._conn.commit()

    def connect(self) -> None:
        self._conn = sqlite3.connect(self._db_path)
        self._cursor = self._conn.cursor()

    def close(self) -> None:
        self._cursor.close()
        self._conn.close()

    @contextmanager
    def connection(self) -> Generator[sqlite3.Connection, None, None]:
        self.connect()
        try:
            yield self._conn
        finally:
            self.close()
