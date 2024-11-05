ALTER TABLE actors ALTER COLUMN as_url TYPE jsonb USING ('"' || as_url || '"')::jsonb;
