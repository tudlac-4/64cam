CREATE TYPE event_type AS ENUM (
    'motion',
    'object_detection',
    'connectivity',
    'recording_marker',
    'custom'
);

CREATE TABLE events (
    id          UUID        NOT NULL DEFAULT gen_random_uuid(),
    camera_id   UUID        NOT NULL,
    node_id     UUID        NOT NULL,
    type        event_type  NOT NULL,
    payload     JSONB       NOT NULL DEFAULT '{}',
    occurred_at TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, occurred_at)
) PARTITION BY RANGE (occurred_at);

CREATE INDEX idx_events_camera_occurred ON events(camera_id, occurred_at DESC);
CREATE INDEX idx_events_node_occurred   ON events(node_id,   occurred_at DESC);
CREATE INDEX idx_events_type_occurred   ON events(type,      occurred_at DESC);

-- Fallback: create a catch-all partition when pg_partman is not installed (dev/CI)
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_partman') THEN
        EXECUTE 'CREATE TABLE events_default PARTITION OF events DEFAULT';
    END IF;
END $$;
