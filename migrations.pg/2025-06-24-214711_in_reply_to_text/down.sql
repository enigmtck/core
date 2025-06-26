-- Convert as_in_reply_to from TEXT back to JSONB
ALTER TABLE objects 
ADD COLUMN as_in_reply_to_temp JSONB;

UPDATE objects 
SET as_in_reply_to_temp = CASE 
    WHEN as_in_reply_to IS NULL THEN NULL
    ELSE to_jsonb(as_in_reply_to)
END;

ALTER TABLE objects 
DROP COLUMN as_in_reply_to;

ALTER TABLE objects 
RENAME COLUMN as_in_reply_to_temp TO as_in_reply_to;

CREATE INDEX idx_in_reply_to ON objects USING gin(as_in_reply_to);
DROP INDEX IF EXISTS idx_as_in_reply_to;
