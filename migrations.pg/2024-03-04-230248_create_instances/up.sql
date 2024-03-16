CREATE TABLE instances (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  domain_name VARCHAR NOT NULL UNIQUE,
  json JSONB,
  blocked BOOLEAN NOT NULL DEFAULT 'false'
);

SELECT diesel_manage_updated_at('instances');
