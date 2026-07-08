-- Dev fixtures only — NEVER run in production
-- Run after migrations: psql $DATABASE_URL -f seeds/dev_fixtures.sql

INSERT INTO nodes (name, api_key_hash, status, version) VALUES
    ('dev-node-01', 'dev_key_hash_aaa', 'approved', '0.1.0'),
    ('dev-node-02', 'dev_key_hash_bbb', 'pending',  '0.1.0');

INSERT INTO cameras (node_id, name, rtsp_url, stream_path)
SELECT id, 'Front Door', 'rtsp://192.168.1.100:554/stream1', 'front-door'
  FROM nodes WHERE name = 'dev-node-01';

INSERT INTO cameras (node_id, name, rtsp_url, stream_path)
SELECT id, 'Back Yard', 'rtsp://192.168.1.101:554/stream1', 'back-yard'
  FROM nodes WHERE name = 'dev-node-01';
