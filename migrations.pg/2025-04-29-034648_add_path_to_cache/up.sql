ALTER TABLE cache ADD COLUMN path TEXT;

-- Populate the path for existing records, assuming they were stored directly under cache/ using their UUID.
UPDATE cache SET path = uuid WHERE path IS NULL;
