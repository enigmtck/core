ALTER TABLE olm_sessions DROP COLUMN profile_id;
ALTER TABLE olm_sessions DROP COLUMN remote_id;
ALTER TABLE olm_sessions ADD COLUMN encrypted_session_id INT NOT NULL;
ALTER TABLE olm_sessions ADD CONSTRAINT fk_olm_sessions_encrypted_sessions FOREIGN KEY(encrypted_session_id) REFERENCES encrypted_sessions(id);
