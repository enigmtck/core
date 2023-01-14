CREATE TABLE timeline (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  tag JSONB,
  attributed_to VARCHAR NOT NULL,
  remote_actor_id INT NOT NULL,
  ap_id VARCHAR NOT NULL,
  kind VARCHAR NOT NULL,
  url VARCHAR,
  published VARCHAR,
  replies JSONB,
  in_reply_to VARCHAR,
  content VARCHAR NOT NULL,
  ap_public BOOLEAN NOT NULL DEFAULT false,
  CONSTRAINT fk_timeline_remote_actor FOREIGN KEY(remote_actor_id) REFERENCES remote_actors(id)
);

SELECT diesel_manage_updated_at('timeline');

CREATE TABLE timeline_cc (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  timeline_id INT NOT NULL,
  ap_id VARCHAR NOT NULL,
  CONSTRAINT fk_timeline_cc_timeline FOREIGN KEY(timeline_id) REFERENCES timeline(id)
);

SELECT diesel_manage_updated_at('timeline_cc');

CREATE TABLE timeline_to (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  timeline_id INT NOT NULL,
  ap_id VARCHAR NOT NULL,
  CONSTRAINT fk_timeline_to_timeline FOREIGN KEY(timeline_id) REFERENCES timeline(id)
);

SELECT diesel_manage_updated_at('timeline_to');
