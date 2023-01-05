CREATE TABLE remote_encrypted_sessions (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  profile_id INT NOT NULL,
  actor VARCHAR NOT NULL,
  kind VARCHAR NOT NULL,
  ap_id VARCHAR NOT NULL UNIQUE,
  ap_to VARCHAR NOT NULL,
  attributed_to VARCHAR NOT NULL,
  instrument JSONB NOT NULL,
  reference VARCHAR,
  CONSTRAINT fk_profile_sessions FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

SELECT diesel_manage_updated_at('remote_encrypted_sessions');
