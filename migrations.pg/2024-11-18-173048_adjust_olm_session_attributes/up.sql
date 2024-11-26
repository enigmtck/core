DELETE FROM olm_sessions;
ALTER TABLE olm_sessions ADD COLUMN ap_conversation TEXT NOT NULL;
ALTER TABLE olm_sessions ADD COLUMN owner_id INT NOT NULL;
ALTER TABLE olm_sessions ADD CONSTRAINT fk_olm_sessions_owner_id FOREIGN KEY (owner_id) REFERENCES actors (id);
