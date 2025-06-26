-- Convert as_in_reply_to from JSONB to TEXT
-- Extract the string value from JSON and store as TEXT
ALTER TABLE objects 
ADD COLUMN as_in_reply_to_temp TEXT;

UPDATE objects 
SET as_in_reply_to_temp = CASE 
    WHEN as_in_reply_to IS NULL THEN NULL
    WHEN jsonb_typeof(as_in_reply_to) = 'string' THEN as_in_reply_to #>> '{}'
    ELSE as_in_reply_to::text
END;

ALTER TABLE objects 
DROP COLUMN as_in_reply_to;

ALTER TABLE objects 
RENAME COLUMN as_in_reply_to_temp TO as_in_reply_to;

CREATE INDEX idx_as_in_reply_to ON objects(as_in_reply_to);
DROP INDEX IF EXISTS idx_in_reply_to;
