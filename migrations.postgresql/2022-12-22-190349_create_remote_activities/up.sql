CREATE TABLE remote_activities (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  profile_id INT NOT NULL,
  context JSONB,
  kind VARCHAR NOT NULL,
  ap_id VARCHAR NOT NULL,
  ap_to JSONB,
  cc JSONB,
  actor VARCHAR NOT NULL,
  published VARCHAR,
  ap_object JSONB,
  CONSTRAINT fk_profile_remote_activities FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

SELECT diesel_manage_updated_at('remote_activities');
