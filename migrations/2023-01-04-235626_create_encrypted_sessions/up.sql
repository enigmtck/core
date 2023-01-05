CREATE TABLE encrypted_sessions (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  profile_id INT NOT NULL,
  ap_to VARCHAR NOT NULL,
  attributed_to VARCHAR NOT NULL,
  instrument JSONB NOT NULL,
  reference VARCHAR,
  uuid VARCHAR NOT NULL,
  CONSTRAINT fk_profile_encrypted_sessions FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

SELECT diesel_manage_updated_at('encrypted_sessions');
