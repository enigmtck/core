CREATE TABLE followers (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  profile_id INT NOT NULL,
  ap_id VARCHAR NOT NULL UNIQUE,
  actor VARCHAR NOT NULL,
  followed_ap_id VARCHAR NOT NULL,
  CONSTRAINT fk_profile_followers FOREIGN KEY(profile_id) REFERENCES profiles(id),
  CONSTRAINT uniq_actor_followed UNIQUE (actor, followed_ap_id)
);


SELECT diesel_manage_updated_at('followers');
