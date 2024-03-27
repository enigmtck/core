DELETE FROM timeline WHERE kind != 'note';

CREATE TYPE timeline_type AS ENUM ('note', 'question');

ALTER TABLE timeline ALTER COLUMN kind TYPE text USING kind::text;
ALTER TABLE timeline ALTER COLUMN kind TYPE timeline_type USING kind::timeline_type;

ALTER TABLE timeline ADD COLUMN end_time TIMESTAMPTZ;
ALTER TABLE timeline ADD COLUMN one_of JSONB;
ALTER TABLE timeline ADD COLUMN any_of JSONB;
ALTER TABLE timeline ADD COLUMN voters_count INTEGER;

