CREATE TABLE activities (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  profile_id INT NOT NULL,
  kind VARCHAR NOT NULL,
  uuid VARCHAR NOT NULL,
  ap_to JSONB,
  cc JSONB,
  target_note_id INT,
  target_remote_note_id INT,
  target_profile_id INT,
  CONSTRAINT fk_profile_activities FOREIGN KEY (profile_id) REFERENCES profiles(id),
  CONSTRAINT target_not_null CHECK (NOT (target_note_id IS NULL AND target_remote_note_id IS NULL AND target_profile_id IS NULL))
);

SELECT diesel_manage_updated_at('activities');
