ALTER TABLE activities DROP CONSTRAINT target_not_null;

ALTER TABLE activities DROP COLUMN target_actor_id;
ALTER TABLE activities DROP COLUMN actor_id;
ALTER TABLE followers DROP COLUMN actor_id;
ALTER TABLE leaders DROP COLUMN actor_id;

DROP TABLE actors;
DROP TYPE actor_type;

ALTER TABLE activities
  ADD CONSTRAINT target_not_null
  CHECK (
    NOT (
      target_note_id IS NULL
      AND target_remote_note_id IS NULL
      AND target_profile_id IS NULL
      AND target_remote_actor_id IS NULL
      AND target_activity_id IS NULL
      AND target_object_id IS NULL));

