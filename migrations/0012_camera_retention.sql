ALTER TABLE cameras
    ADD COLUMN retention_policy_id UUID REFERENCES retention_policies(id);
