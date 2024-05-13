BEGIN;

CREATE TABLE IF NOT EXISTS auditor (
    id INT GENERATED ALWAYS AS IDENTITY,
    record_id TEXT NOT NULL UNIQUE,
    meta JSONB,
    components JSONB,
    start_time TIMESTAMPTZ NOT NULL,
    stop_time TIMESTAMPTZ,
    runtime BIGINT,
    updated_at  TIMESTAMPTZ NOT NULL
);

INSERT INTO auditor (record_id, meta, components, start_time, stop_time, runtime, updated_at)

SELECT a.record_id,
                          m.meta::jsonb AS meta,
                          css.components::jsonb AS Components,
                          a.start_time,
                          a.stop_time,
                          a.runtime,
                          a.updated_at
                   FROM accounting a
                   LEFT JOIN (
    WITH subquery AS (
        SELECT m.record_id as record_id, m.key as key, array_agg(m.value) as values
        FROM meta as m
        GROUP BY m.record_id, m.key
    )               
    SELECT s.record_id as record_id, jsonb_object_agg(s.key, s.values) as meta
    FROM subquery as s
    GROUP BY s.record_id
) m ON m.record_id = a.record_id
                   LEFT JOIN (
                       WITH subquery AS (
                          SELECT 
                              c.id as cid,
                              COALESCE(array_agg(row(s.name, s.value)::score) FILTER (WHERE s.name IS NOT NULL AND s.value IS NOT NULL), '{}'::score[]) as scores
                          FROM components as c
                          LEFT JOIN components_scores as cs
                          ON c.id = cs.component_id
                          LEFT JOIN scores as s
                          ON cs.score_id = s.id
                          GROUP BY c.id
                       )
                       SELECT rc.record_id as id, jsonb_agg(row(c.name, c.amount, sq.scores)::component) as components
                       FROM records_components AS rc
                       LEFT JOIN components as c
                       ON rc.component_id = c.id
                       LEFT JOIN subquery AS sq
                       ON sq.cid = rc.component_id
                       GROUP BY rc.record_id
                   ) css ON css.id = a.id;

DROP TABLE IF EXISTS records_components;
DROP TABLE IF EXISTS components_scores;
DROP TABLE IF EXISTS meta;
DROP TABLE IF EXISTS scores;
DROP TABLE IF EXISTS components;
DROP TABLE IF EXISTS accounting;

COMMIT;
