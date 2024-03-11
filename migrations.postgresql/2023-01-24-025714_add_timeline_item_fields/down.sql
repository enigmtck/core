ALTER TABLE timeline
      DROP COLUMN summary,
      DROP COLUMN ap_sensitive,
      DROP COLUMN atom_uri,
      DROP COLUMN in_reply_to_atom_uri,
      DROP COLUMN conversation,
      DROP COLUMN content_map,
      DROP COLUMN attachment;
