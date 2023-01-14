CREATE TABLE processing_queue (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  profile_id INT NOT NULL,
  ap_id VARCHAR NOT NULL,
  ap_to JSONB NOT NULL,
  cc JSONB,
  attributed_to VARCHAR NOT NULL,
  kind VARCHAR NOT NULL,
  ap_object JSONB NOT NULL,
  processed boolean NOT NULL,
  CONSTRAINT fk_profile_processing_queue FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

SELECT diesel_manage_updated_at('processing_queue');
