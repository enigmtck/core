CREATE TABLE remote_announces (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  context VARCHAR,
  kind VARCHAR NOT NULL,
  ap_id VARCHAR NOT NULL UNIQUE,
  actor VARCHAR NOT NULL,
  ap_to JSONB,
  cc JSONB,
  published VARCHAR NOT NULL,
  ap_object JSONB NOT NULL,
  timeline_id INT,
  revoked BOOLEAN NOT NULL DEFAULT false,
  CONSTRAINT fk_timeline_remote_announces FOREIGN KEY(timeline_id) REFERENCES timeline(id) 
);

SELECT diesel_manage_updated_at('remote_announces');
