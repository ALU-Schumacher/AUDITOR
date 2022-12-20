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
    site_id     TEXT,
    user_id     TEXT,
    group_id    TEXT,
	components  component[],
	start_time  TIMESTAMPTZ NOT NULL,
	stop_time   TIMESTAMPTZ,
	runtime     BIGINT,
    updated_at  TIMESTAMPTZ NOT NULL
);
