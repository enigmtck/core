CREATE TABLE leaders (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  profile_id INT NOT NULL,
  actor VARCHAR NOT NULL,
  leader_ap_id VARCHAR NOT NULL,
  uuid VARCHAR NOT NULL UNIQUE,
  accept_ap_id VARCHAR,
  accepted BOOLEAN,
  CONSTRAINT fk_profile_leaders FOREIGN KEY(profile_id) REFERENCES profiles(id),
  CONSTRAINT uniq_actor_leader UNIQUE (actor, leader_ap_id)
);

SELECT diesel_manage_updated_at('leaders');
