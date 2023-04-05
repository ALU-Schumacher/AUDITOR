import sqlite3

from contextlib import contextmanager


class StateDB(object):
    """
    A simple wrapper for an sqlite database for persistant storage of the last
    job id for each scheduler name/record prefix combination.
    The path to the database is set in the config file with the `state_db` key.
    If the database does not exist, it will be created.
    """
    def __init__(self, db_path):
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

    def get(self, schedd, prefix):
        self._cursor.execute(
            "SELECT cluster, proc FROM last_jobs WHERE schedd = ? AND prefix = ?",
            (schedd, prefix),
        )
        row = self._cursor.fetchone()
        if row:
            return row
        return None

    def set(self, schedd, prefix, cluster, proc):
        self._cursor.execute(
            "INSERT OR REPLACE INTO last_jobs VALUES (?, ?, ?, ?)",
            (schedd, prefix, cluster, proc),
        )
        self._conn.commit()

    def connect(self):
        self._conn = sqlite3.connect(self._db_path)
        self._cursor = self._conn.cursor()

    def close(self):
        self._cursor.close()
        self._conn.close()

    @contextmanager
    def connection(self):
        self.connect()
        try:
            yield self._conn
        finally:
            self.close()
