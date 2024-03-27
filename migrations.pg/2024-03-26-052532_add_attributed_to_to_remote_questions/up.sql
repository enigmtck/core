ALTER TABLE remote_questions ADD COLUMN attributed_to VARCHAR NOT NULL COLLATE "case_insensitive" DEFAULT 'placeholder';
