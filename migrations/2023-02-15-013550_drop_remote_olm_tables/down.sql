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
