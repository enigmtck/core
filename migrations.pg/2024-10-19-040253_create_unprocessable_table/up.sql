CREATE TABLE unprocessable (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  raw JSONB NOT NULL
);

SELECT diesel_manage_updated_at('unprocessable');
