CREATE TABLE notes (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR UNIQUE NOT NULL,
  profile_id INT NOT NULL,
  kind VARCHAR NOT NULL,
  ap_to JSONB NOT NULL,
  cc JSONB,
  tag JSONB,
  attributed_to VARCHAR NOT NULL,
  in_reply_to VARCHAR,
  content VARCHAR NOT NULL,
  CONSTRAINT fk_profile_notes FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

SELECT diesel_manage_updated_at('notes');
