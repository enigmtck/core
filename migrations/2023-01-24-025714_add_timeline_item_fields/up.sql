ALTER TABLE timeline
      ADD COLUMN summary VARCHAR,
      ADD COLUMN ap_sensitive boolean,
      ADD COLUMN atom_uri VARCHAR,
      ADD COLUMN in_reply_to_atom_uri VARCHAR,
      ADD COLUMN conversation VARCHAR,
      ADD COLUMN content_map JSONB,
      ADD COLUMN attachment JSONB;
