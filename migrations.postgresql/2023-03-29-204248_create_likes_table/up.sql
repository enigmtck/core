CREATE TABLE likes (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR NOT NULL,
  profile_id INT,
  ap_to VARCHAR NOT NULL,
  actor VARCHAR NOT NULL,
  object_ap_id VARCHAR NOT NULL
);

CREATE UNIQUE INDEX idx_likes_actor_object_ap_id ON likes (actor, object_ap_id);

SELECT diesel_manage_updated_at('likes');
