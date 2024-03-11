CREATE TABLE profiles (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR UNIQUE NOT NULL,
  username VARCHAR UNIQUE NOT NULL,
  display_name VARCHAR NOT NULL,
  summary VARCHAR,
  public_key VARCHAR NOT NULL,
  private_key VARCHAR NOT NULL
);

CREATE UNIQUE INDEX idx_username_unique_lower ON profiles (lower(username));

SELECT diesel_manage_updated_at('profiles');
