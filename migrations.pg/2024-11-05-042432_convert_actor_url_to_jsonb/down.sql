ALTER TABLE actors ALTER COLUMN as_url TYPE TEXT USING trim(both '"' from as_url::text);
