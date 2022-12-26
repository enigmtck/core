CREATE TABLE remote_notes (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  profile_id INT NOT NULL,
  ap_id VARCHAR NOT NULL,
  published VARCHAR,
  url VARCHAR,
  attributed_to VARCHAR,
  ap_to JSONB,
  cc JSONB,
  content VARCHAR NOT NULL,
  attachment JSONB,
  tag JSONB,
  replies JSONB,
  signature JSONB,
  CONSTRAINT fk_profile_remote_notes FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

SELECT diesel_manage_updated_at('remote_notes');
