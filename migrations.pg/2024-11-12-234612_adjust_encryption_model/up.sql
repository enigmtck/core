ALTER TABLE olm_sessions DROP COLUMN encrypted_session_id;
DELETE FROM olm_sessions;
ALTER TABLE olm_sessions ADD COLUMN owner_as_id TEXT NOT NULL COLLATE "case_insensitive";
ALTER TABLE olm_sessions ADD CONSTRAINT fk_actors_owner_as_id FOREIGN KEY (owner_as_id) REFERENCES actors (as_id) ON DELETE CASCADE;
ALTER TABLE olm_sessions ADD COLUMN remote_as_id TEXT NOT NULL COLLATE "case_insensitive";
ALTER TABLE olm_sessions ADD CONSTRAINT fk_actors_remote_as_id FOREIGN KEY (remote_as_id) REFERENCES actors (as_id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS olm_sessions_owner_as_id ON olm_sessions (owner_as_id);
CREATE INDEX IF NOT EXISTS olm_sessions_remote_as_id ON olm_sessions (remote_as_id);
CREATE INDEX IF NOT EXISTS olm_sessions_uuid ON olm_sessions (uuid);
CREATE INDEX IF NOT EXISTS olm_sessions_owner_as_id_remote_as_id ON olm_sessions (owner_as_id, remote_as_id);

DELETE FROM vault;
ALTER TABLE vault ADD COLUMN owner_as_id TEXT NOT NULL COLLATE "case_insensitive";
ALTER TABLE vault ADD CONSTRAINT fk_actors_owner_as_id FOREIGN KEY (owner_as_id) REFERENCES actors (as_id) ON DELETE CASCADE;
ALTER TABLE vault ADD COLUMN activity_id INT NOT NULL;
ALTER TABLE vault ADD CONSTRAINT fk_activities_activity_id FOREIGN KEY (activity_id) REFERENCES activities (id) ON DELETE CASCADE;
ALTER TABLE vault ADD COLUMN data TEXT NOT NULL;
ALTER TABLE vault DROP COLUMN profile_id;
ALTER TABLE vault DROP COLUMN remote_actor;
ALTER TABLE vault DROP COLUMN outbound;
ALTER TABLE vault DROP COLUMN encrypted_data;
CREATE INDEX IF NOT EXISTS vault_uuid ON vault (uuid);
CREATE INDEX IF NOT EXISTS vault_owner_as_id ON vault (owner_as_id);
CREATE INDEX IF NOT EXISTS vault_activity_id ON vault (activity_id);
