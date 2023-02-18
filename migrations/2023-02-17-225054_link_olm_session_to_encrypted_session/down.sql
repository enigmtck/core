ALTER TABLE olm_sessions ADD COLUMN profile_id INT;
ALTER TABLE olm_sessions ADD COLUMN remote_id VARCHAR;
ALTER TABLE olm_sessions DROP COLUMN encrypted_session_id;
ALTER TABLE olm_sessions ADD CONSTRAINT fk_olm_sessions_profile FOREIGN KEY(profile_id) REFERENCES profiles(id);
