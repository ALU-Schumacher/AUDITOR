-- Add migration script here
CREATE TABLE records (
	id          TEXT NOT NULL UNIQUE,
    PRIMARY KEY (id),
    record      BLOB NOT NULL,
);
