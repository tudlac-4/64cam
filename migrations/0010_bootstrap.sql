INSERT INTO retention_policies (name, keep_days, is_default) VALUES
    ('30_days',  30,  true),
    ('7_days',   7,   false),
    ('90_days',  90,  false),
    ('365_days', 365, false);

INSERT INTO roles (name, permissions) VALUES
    ('admin',    '{"all": true}'),
    ('operator', '{"cameras": ["read", "write"], "recordings": ["read"], "events": ["read"]}'),
    ('viewer',   '{"cameras": ["read"], "recordings": ["read"], "events": ["read"]}');
