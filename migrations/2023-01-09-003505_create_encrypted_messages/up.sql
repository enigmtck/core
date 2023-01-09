CREATE TABLE encrypted_messages (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR UNIQUE NOT NULL,
  profile_id INT NOT NULL,
  ap_to JSONB NOT NULL,
  attributed_to VARCHAR NOT NULL,
  cc JSONB,
  in_reply_to VARCHAR,
  encrypted_content VARCHAR NOT NULL,
  CONSTRAINT fk_profile_encrypted_messages FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

SELECT diesel_manage_updated_at('encrypted_messages');
