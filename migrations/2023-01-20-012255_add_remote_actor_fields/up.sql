ALTER TABLE remote_actors
      ADD COLUMN featured VARCHAR,
      ADD COLUMN featured_tags VARCHAR,
      ADD COLUMN url VARCHAR,
      ADD COLUMN manually_approves_followers boolean,
      ADD COLUMN published VARCHAR,
      ADD COLUMN tag JSONB,
      ADD COLUMN attachment JSONB,
      ADD COLUMN endpoints JSONB,
      ADD COLUMN icon JSONB,
      ADD COLUMN image JSONB;
