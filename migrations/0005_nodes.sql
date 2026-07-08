CREATE TYPE node_status AS ENUM ('pending', 'approved', 'rejected');

CREATE TABLE nodes (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name         TEXT        NOT NULL,
    api_key_hash TEXT        NOT NULL UNIQUE,
    status       node_status NOT NULL DEFAULT 'pending',
    last_seen_at TIMESTAMPTZ,
    ip_addr      INET,
    version      TEXT,
    metadata     JSONB       NOT NULL DEFAULT '{}',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_nodes_status       ON nodes(status);
CREATE INDEX idx_nodes_api_key_hash ON nodes(api_key_hash);
