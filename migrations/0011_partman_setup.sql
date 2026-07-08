-- Configure pg_partman for automatic monthly partition maintenance.
-- No-ops safely when pg_partman is not installed (dev/CI uses default partitions).
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_partman') THEN
        PERFORM partman.create_parent(
            p_parent_table => 'public.recordings',
            p_control      => 'started_at',
            p_interval     => 'monthly'
        );
        PERFORM partman.create_parent(
            p_parent_table => 'public.events',
            p_control      => 'occurred_at',
            p_interval     => 'monthly'
        );
    END IF;
END $$;
