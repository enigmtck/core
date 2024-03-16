CREATE TABLE cache (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR UNIQUE NOT NULL,
  url VARCHAR UNIQUE NOT NULL,
  media_type VARCHAR NOT NULL,
  height INT NOT NULL,
  width INT NOT NULL,
  blurhash VARCHAR
);

CREATE INDEX idx_cache_url ON cache (url);

SELECT diesel_manage_updated_at('cache');
