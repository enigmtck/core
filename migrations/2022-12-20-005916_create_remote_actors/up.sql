CREATE TABLE remote_actors (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  context JSONB NOT NULL,
  kind VARCHAR NOT NULL,
  ap_id VARCHAR UNIQUE NOT NULL,
  name VARCHAR NOT NULL,
  preferred_username VARCHAR,
  summary VARCHAR,
  inbox VARCHAR NOT NULL,
  outbox VARCHAR NOT NULL,
  followers VARCHAR NOT NULL,
  following VARCHAR NOT NULL,
  liked VARCHAR,
  public_key JSONB
);

SELECT diesel_manage_updated_at('remote_actors');
