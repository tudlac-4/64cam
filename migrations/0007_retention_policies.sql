CREATE TABLE retention_policies (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT        NOT NULL UNIQUE,
    keep_days  INTEGER     NOT NULL CHECK (keep_days > 0),
    is_default BOOLEAN     NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Enforce at most one default policy
CREATE UNIQUE INDEX idx_retention_policies_single_default
    ON retention_policies(is_default) WHERE is_default = true;
