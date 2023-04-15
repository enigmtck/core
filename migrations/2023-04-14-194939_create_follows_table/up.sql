CREATE TABLE follows (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR NOT NULL,
  profile_id INT,
  ap_object VARCHAR NOT NULL,
  actor VARCHAR NOT NULL
);

SELECT diesel_manage_updated_at('follows');
