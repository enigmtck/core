ALTER TABLE timeline ALTER COLUMN kind TYPE text USING kind::text;
ALTER TABLE timeline ALTER COLUMN kind TYPE note_type USING kind::note_type;

DROP TYPE timeline_type;

ALTER TABLE timeline DROP COLUMN end_time;
ALTER TABLE timeline DROP COLUMN one_of;
ALTER TABLE timeline DROP COLUMN any_of;
ALTER TABLE timeline DROP COLUMN voters_count;
