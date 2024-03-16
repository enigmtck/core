ALTER TABLE remote_actors
      ADD COLUMN also_known_as JSONB,
      ADD COLUMN discoverable BOOLEAN,
      ADD COLUMN capabilities JSONB;
