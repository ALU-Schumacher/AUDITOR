-- Create accounting table
CREATE TYPE score AS (
    name        TEXT,
    value       double precision
);

CREATE TYPE component AS (
    name        TEXT,
    amount      BIGINT,
    scores      score[]
);

CREATE TABLE accounting (
	record_id   TEXT NOT NULL UNIQUE,
    PRIMARY KEY (record_id),
	components  component[],
	start_time  TIMESTAMPTZ NOT NULL,
	stop_time   TIMESTAMPTZ,
	runtime     BIGINT,
    updated_at  TIMESTAMPTZ NOT NULL
);

CREATE TABLE meta (
    record_id  TEXT NOT NULL,
    key        TEXT,
    value      TEXT[]
);
