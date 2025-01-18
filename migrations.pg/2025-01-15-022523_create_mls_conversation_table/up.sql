CREATE TABLE mls_group_conversations (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid TEXT NOT NULL,
  actor_id INTEGER NOT NULL REFERENCES actors (id) ON DELETE CASCADE,
  conversation TEXT NOT NULL,
  mls_group TEXT UNIQUE NOT NULL
);

CREATE INDEX idx_mls_group_conversations_created_at_asc ON mls_group_conversations USING btree (created_at ASC);
CREATE INDEX idx_mls_group_conversations_actor_id ON mls_group_conversations USING btree (actor_id);
CREATE INDEX idx_mls_group_conversations_uuid ON mls_group_conversations USING btree (uuid);
CREATE UNIQUE INDEX uniq_mls_group_conversations_actor_id_conversation ON mls_group_conversations (actor_id, conversation);

SELECT diesel_manage_updated_at('mls_group_conversations');
