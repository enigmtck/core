ALTER TABLE olm_sessions ADD COLUMN encrypted_session_id INT;
ALTER TABLE olm_sessions DROP COLUMN owner_as_id; 
ALTER TABLE olm_sessions DROP COLUMN remote_as_id;

ALTER TABLE vault DROP COLUMN owner_as_id;
ALTER TABLE vault DROP COLUMN activity_id;
ALTER TABLE vault DROP COLUMN data;
ALTER TABLE vault ADD COLUMN profile_id INT NOT NULL;
ALTER TABLE vault ADD COLUMN remote_actor TEXT NOT NULL;
ALTER TABLE vault ADD COLUMN outbound BOOLEAN NOT NULL DEFAULT(false);
ALTER TABLE vault ADD COLUMN encrypted_data TEXT NOT NULL;
