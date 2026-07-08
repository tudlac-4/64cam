-- Allow efficient single-recording lookup by id alone (without knowing started_at).
-- The primary key index is on (id, started_at) for partition pruning; this secondary
-- index lets the coordinator find a recording by id without scanning all partitions.
CREATE INDEX IF NOT EXISTS recordings_id_idx ON recordings (id);
