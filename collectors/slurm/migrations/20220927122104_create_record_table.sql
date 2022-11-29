CREATE TABLE IF NOT EXISTS records (
	id          TEXT NOT NULL PRIMARY KEY,
    record      BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS lastcheck (
    lastcheck   DATETIME NOT NULL PRIMARY KEY,
	jobid       TEXT NOT NULL
);
