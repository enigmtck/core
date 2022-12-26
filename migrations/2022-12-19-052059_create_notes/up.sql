CREATE TABLE notes (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR UNIQUE NOT NULL,
  profile_id INT NOT NULL,
  content VARCHAR NOT NULL,
  ap_to JSONB NOT NULL,
  ap_tag JSONB,
  CONSTRAINT fk_profile_notes FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

SELECT diesel_manage_updated_at('notes');
