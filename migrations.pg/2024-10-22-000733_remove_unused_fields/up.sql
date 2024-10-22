ALTER TABLE leaders DROP COLUMN profile_id;
ALTER TABLE followers DROP COLUMN profile_id;
ALTER TABLE activities DROP COLUMN profile_id;
ALTER TABLE activities DROP COLUMN target_note_id;
ALTER TABLE activities DROP COLUMN target_remote_note_id;
ALTER TABLE activities DROP COLUMN target_profile_id;
ALTER TABLE activities DROP COLUMN target_remote_actor_id;
ALTER TABLE activities DROP COLUMN target_remote_question_id;

