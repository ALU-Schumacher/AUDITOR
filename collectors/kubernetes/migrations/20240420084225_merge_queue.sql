-- Add migration script here
CREATE TABLE IF NOT EXISTS mergequeue (
    rid         VARCHAR(256) NOT NULL,
    record      BLOB NOT NULL,
    retry       INTEGER NOT NULL,
    updated     INTEGER NOT NULL,
    complete    BOOLEAN NOT NULL
);

CREATE TABLE IF NOT EXISTS lastcheck (
    time    DATETIME NOT NULL
);
