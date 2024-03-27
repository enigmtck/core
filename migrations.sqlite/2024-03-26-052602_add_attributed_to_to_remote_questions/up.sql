ALTER TABLE remote_questions ADD COLUMN attributed_to TEXT NOT NULL COLLATE NOCASE DEFAULT 'placeholder';
