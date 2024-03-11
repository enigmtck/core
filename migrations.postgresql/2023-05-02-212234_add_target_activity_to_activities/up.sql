ALTER TABLE activities ADD COLUMN target_activity_id INT;

ALTER TABLE activities DROP CONSTRAINT target_not_null;
ALTER TABLE activities ADD CONSTRAINT target_not_null CHECK (NOT (target_note_id IS NULL AND target_remote_note_id IS NULL AND target_profile_id IS NULL AND target_activity_id IS NULL));
