CREATE TABLE vault (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR NOT NULL,
  profile_id INT NOT NULL,
  encrypted_data VARCHAR NOT NULL,
  CONSTRAINT fk_vault_profile FOREIGN KEY(profile_id) REFERENCES profiles(id) 
);

SELECT diesel_manage_updated_at('vault');
