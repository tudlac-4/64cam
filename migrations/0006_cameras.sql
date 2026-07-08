CREATE TABLE cameras (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id     UUID        NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    rtsp_url    TEXT        NOT NULL,
    stream_path TEXT,
    enabled     BOOLEAN     NOT NULL DEFAULT true,
    metadata    JSONB       NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_cameras_node_id         ON cameras(node_id);
CREATE INDEX idx_cameras_node_id_enabled ON cameras(node_id) WHERE enabled = true;
