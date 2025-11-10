-- Add index on extract_parent_id(as_in_reply_to) to optimize orphan-finding queries in closure table trigger
-- This dramatically improves performance of object insertion (from ~6 seconds to milliseconds)
CREATE INDEX IF NOT EXISTS idx_objects_parent_id ON objects (extract_parent_id(as_in_reply_to));
