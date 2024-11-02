CREATE INDEX idx_actors_public_key ON actors USING gin (as_public_key);
