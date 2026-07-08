CREATE SCHEMA IF NOT EXISTS partman;
DO $$ BEGIN
    CREATE EXTENSION IF NOT EXISTS pg_partman SCHEMA partman;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'pg_partman not available — partition maintenance will be manual';
END $$;
