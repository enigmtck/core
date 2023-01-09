CREATE TABLE remote_encrypted_messages (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  profile_id INT NOT NULL,
  ap_id VARCHAR NOT NULL,
  ap_to JSONB NOT NULL,
  cc JSONB,
  attributed_to VARCHAR NOT NULL,
  published VARCHAR NOT NULL,
  in_reply_to VARCHAR,
  encrypted_content JSONB NOT NULL,
  CONSTRAINT fk_profile_remote_encrypted_messages FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

SELECT diesel_manage_updated_at('remote_encrypted_messages');
