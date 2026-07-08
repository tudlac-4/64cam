CREATE TABLE recordings (
    id                  UUID        NOT NULL DEFAULT gen_random_uuid(),
    camera_id           UUID        NOT NULL,
    node_id             UUID        NOT NULL,
    storage_uri         TEXT        NOT NULL,
    started_at          TIMESTAMPTZ NOT NULL,
    ended_at            TIMESTAMPTZ,
    duration_secs       INTEGER,
    size_bytes          BIGINT,
    retention_policy_id UUID        REFERENCES retention_policies(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, started_at)
) PARTITION BY RANGE (started_at);

CREATE INDEX idx_recordings_camera_started ON recordings(camera_id, started_at DESC);
CREATE INDEX idx_recordings_node_started   ON recordings(node_id,   started_at DESC);

-- Fallback: create a catch-all partition when pg_partman is not installed (dev/CI)
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_partman') THEN
        EXECUTE 'CREATE TABLE recordings_default PARTITION OF recordings DEFAULT';
    END IF;
END $$;
