CREATE TABLE mls_key_packages (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid TEXT NOT NULL,
  actor_id INTEGER NOT NULL,
  key_data TEXT NOT NULL,
  distributed BOOLEAN NOT NULL DEFAULT 'f',
  assignee TEXT
);

CREATE INDEX idx_created_at_asc ON mls_key_packages USING btree (created_at ASC);
CREATE INDEX idx_actor_id ON mls_key_packages USING btree (actor_id);
CREATE INDEX idx_uuid ON mls_key_packages USING btree (uuid);

SELECT diesel_manage_updated_at('mls_key_packages');
