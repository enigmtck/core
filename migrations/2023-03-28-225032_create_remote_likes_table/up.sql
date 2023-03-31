CREATE TABLE remote_likes (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  ap_id VARCHAR NOT NULL,
  actor VARCHAR NOT NULL,
  object_id VARCHAR NOT NULL
);

CREATE UNIQUE INDEX idx_remote_likes_actor_object_id ON remote_likes (actor, object_id);

SELECT diesel_manage_updated_at('remote_likes');
