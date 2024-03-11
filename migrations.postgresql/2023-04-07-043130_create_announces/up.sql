CREATE TABLE announces (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  profile_id INT,
  uuid VARCHAR NOT NULL,
  actor VARCHAR NOT NULL,
  ap_to JSONB NOT NULL,
  cc JSONB,
  object_ap_id VARCHAR NOT NULL
);

CREATE UNIQUE INDEX idx_announces_actor_object_ap_id ON likes (actor, object_ap_id);

SELECT diesel_manage_updated_at('announces');
