
DROP TABLE note_hashtags;
DROP TABLE remote_note_hashtags;
DROP TABLE remote_actor_hashtags;
DROP TABLE profile_hashtags;
DROP TABLE activities_to;
DROP TABLE activities_cc;
DROP TABLE remote_questions;
DROP TABLE remote_notes;
DROP TABLE remote_actors;
DROP TABLE notes;

ALTER TABLE vault DROP CONSTRAINT fk_vault_profile;
ALTER TABLE remote_encrypted_sessions DROP CONSTRAINT fk_profile_sessions;
ALTER TABLE processing_queue DROP CONSTRAINT fk_profile_processing_queue;
ALTER TABLE leaders DROP CONSTRAINT fk_profile_leaders;
ALTER TABLE followers DROP CONSTRAINT fk_profile_followers;
ALTER TABLE encrypted_sessions DROP CONSTRAINT fk_profile_encrypted_sessions;
ALTER TABLE olm_one_time_keys DROP CONSTRAINT fk_otk_profile;
DROP TABLE profiles;
DROP TABLE likes;
DROP TABLE announces;
