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
    id         INT GENERATED ALWAYS AS IDENTITY,
    PRIMARY KEY (id),
	record_id   TEXT NOT NULL UNIQUE,
	components  component[],
	start_time  TIMESTAMPTZ NOT NULL,
	stop_time   TIMESTAMPTZ,
	runtime     BIGINT,
    updated_at  TIMESTAMPTZ NOT NULL
);

CREATE TABLE meta (
    record_id  TEXT NOT NULL,
    key        TEXT NOT NULL,
    value      TEXT NOT NULL,
    PRIMARY KEY (record_id, key, value),
    FOREIGN KEY (record_id) REFERENCES accounting(record_id)
);

CREATE TABLE components (
    id         INT GENERATED ALWAYS AS IDENTITY,
    PRIMARY KEY (id),
    name       TEXT NOT NULL,
    amount     BIGINT NOT NULL
);

CREATE TABLE scores (
    id         INT GENERATED ALWAYS AS IDENTITY,
    PRIMARY KEY (id),
    name       TEXT NOT NULL,
    value      double precision NOT NULL
);

CREATE TABLE components_scores (
    id              INT GENERATED ALWAYS AS IDENTITY,
    component_id    INT NOT NULL,
    score_id        INT NOT NULL,
    FOREIGN KEY (component_id) REFERENCES components(id),
    FOREIGN KEY (score_id) REFERENCES scores(id)
);

CREATE TABLE records_components (
    record_id       INT NOT NULL,
    component_id    INT NOT NULL,
    FOREIGN KEY (record_id) REFERENCES accounting(id),
    FOREIGN KEY (component_id) REFERENCES components(id)
);
