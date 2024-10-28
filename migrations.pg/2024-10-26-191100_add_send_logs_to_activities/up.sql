ALTER TABLE activities ADD COLUMN log JSONB;
UPDATE activities SET log = '[]' WHERE log IS NULL;
