-- 0012_natural_key_upsert.sql — Add UNIQUE constraints and sequences for natural-key upsert.
-- Allows ON CONFLICT (name) DO UPDATE to replace the unstable ROW_NUMBER() approach
-- that would renumber IDs when upstream adds/removes campuses or faculties.

-- Add sequences so new campuses/faculties get stable generated IDs.
CREATE SEQUENCE IF NOT EXISTS selection.campuses_id_seq;
CREATE SEQUENCE IF NOT EXISTS selection.faculties_id_seq;
ALTER TABLE selection.campuses ALTER COLUMN id SET DEFAULT nextval('selection.campuses_id_seq');
ALTER TABLE selection.faculties ALTER COLUMN id SET DEFAULT nextval('selection.faculties_id_seq');

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'uq_campuses_name'
    ) THEN
        ALTER TABLE selection.campuses ADD CONSTRAINT uq_campuses_name UNIQUE (name);
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'uq_faculties_name'
    ) THEN
        ALTER TABLE selection.faculties ADD CONSTRAINT uq_faculties_name UNIQUE (name);
    END IF;
END $$;
