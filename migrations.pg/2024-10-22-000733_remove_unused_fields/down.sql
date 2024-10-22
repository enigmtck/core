ALTER TABLE leaders ADD COLUMN profile_id INT;
ALTER TABLE followers ADD COLUMN profile_id INT;
ALTER TABLE activities ADD COLUMN profile_id INT;
ALTER TABLE activities ADD COLUMN target_note_id INT;
ALTER TABLE activities ADD COLUMN target_remote_note_id INT;
ALTER TABLE activities ADD COLUMN target_profile_id INT;
ALTER TABLE activities ADD COLUMN target_remote_actor_id INT;
ALTER TABLE activities ADD COLUMN target_remote_question_id INT;
