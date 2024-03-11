ALTER TABLE profiles ADD COLUMN salt VARCHAR;
ALTER TABLE profiles ADD COLUMN client_private_key VARCHAR;
ALTER TABLE profiles ADD COLUMN olm_pickled_account VARCHAR;
ALTER TABLE profiles ADD COLUMN olm_pickled_account_hash VARCHAR;
ALTER TABLE profiles ADD COLUMN olm_identity_key VARCHAR;

CREATE TABLE olm_sessions (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR NOT NULL,
  profile_id INT NOT NULL,
  remote_id VARCHAR NOT NULL,
  session_data VARCHAR NOT NULL,
  session_hash VARCHAR NOT NULL,
  CONSTRAINT fk_olm_sessions_profile FOREIGN KEY(profile_id) REFERENCES profiles(id) 
);

SELECT diesel_manage_updated_at('olm_sessions');

CREATE TABLE olm_one_time_keys (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR NOT NULL,
  profile_id INT NOT NULL,
  olm_id INT NOT NULL,
  key_data VARCHAR NOT NULL,
  distributed BOOLEAN NOT NULL DEFAULT false,
  CONSTRAINT fk_otk_profile FOREIGN KEY(profile_id) REFERENCES profiles(id) 
);

SELECT diesel_manage_updated_at('olm_one_time_keys');

CREATE TABLE remote_olm_one_time_keys (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  attributed_to VARCHAR NOT NULL,
  ap_id VARCHAR NOT NULL,
  ap_to VARCHAR NOT NULL,
  olm_id INT NOT NULL,
  key_data VARCHAR NOT NULL,
  consumed BOOLEAN NOT NULL DEFAULT false
);

SELECT diesel_manage_updated_at('remote_olm_one_time_keys');

CREATE TABLE remote_olm_identity_keys (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  attributed_to VARCHAR NOT NULL,
  ap_id VARCHAR NOT NULL,
  key_data VARCHAR NOT NULL
);

SELECT diesel_manage_updated_at('remote_olm_identity_keys');
