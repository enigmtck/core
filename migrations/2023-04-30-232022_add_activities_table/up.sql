CREATE TYPE activity_type AS ENUM ('create','delete','update','announce', 'like', 'undo', 'follow', 'accept', 'block', 'add', 'remove');

CREATE TABLE activities (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  profile_id INT NOT NULL,
  kind activity_type NOT NULL,
  uuid VARCHAR NOT NULL,
  actor VARCHAR NOT NULL,
  ap_to JSONB,
  cc JSONB,
  target_note_id INT,
  target_remote_note_id INT,
  target_profile_id INT,
  CONSTRAINT fk_profile_activities FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
  CONSTRAINT target_not_null CHECK (NOT (target_note_id IS NULL AND target_remote_note_id IS NULL AND target_profile_id IS NULL))
);

SELECT diesel_manage_updated_at('activities');
