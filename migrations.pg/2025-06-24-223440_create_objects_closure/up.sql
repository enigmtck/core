-- Closure table for objects hierarchy
CREATE TABLE objects_closure (
    ancestor   text NOT NULL,
    descendant text NOT NULL,
    depth      integer NOT NULL CHECK (depth >= 0),
    PRIMARY KEY (ancestor, descendant),
    FOREIGN KEY (ancestor)   REFERENCES objects(as_id) ON DELETE CASCADE,
    FOREIGN KEY (descendant) REFERENCES objects(as_id) ON DELETE CASCADE
);

CREATE INDEX idx_objects_closure_ancestor ON objects_closure(ancestor);
CREATE INDEX idx_objects_closure_descendant ON objects_closure(descendant);

-- Delete orphaned objects
WITH RECURSIVE all_orphans AS (
    SELECT id, as_id, as_in_reply_to
    FROM objects
    WHERE as_in_reply_to IS NOT NULL
      AND NOT EXISTS (
          SELECT 1 FROM objects p 
          WHERE p.as_id = objects.as_in_reply_to
      )
    
    UNION ALL
    
    SELECT o.id, o.as_id, o.as_in_reply_to
    FROM objects o
    JOIN all_orphans a ON o.as_in_reply_to = a.as_id
)
DELETE FROM objects
WHERE id IN (SELECT id FROM all_orphans);

-- Populate closure table with existence check
INSERT INTO objects_closure (ancestor, descendant, depth)
WITH RECURSIVE closure AS (
    SELECT 
        as_id AS descendant, 
        as_id AS ancestor, 
        0 AS depth
    FROM objects
    
    UNION ALL
    
    SELECT 
        c.descendant, 
        p.as_in_reply_to AS ancestor,
        c.depth + 1
    FROM closure c
    JOIN objects p ON c.ancestor = p.as_id
    JOIN objects a ON a.as_id = p.as_in_reply_to  -- Ensure parent exists
    WHERE p.as_in_reply_to IS NOT NULL
)
SELECT ancestor, descendant, depth 
FROM closure;

-- Add foreign key to prevent future orphans
ALTER TABLE objects
ADD CONSTRAINT fk_objects_parent 
FOREIGN KEY (as_in_reply_to) 
REFERENCES objects(as_id) 
ON DELETE SET NULL;

-- 1. Trigger function for INSERT
CREATE OR REPLACE FUNCTION objects_closure_insert_trigger() RETURNS TRIGGER AS $$
BEGIN
    -- Insert self-reference (depth 0)
    INSERT INTO objects_closure (ancestor, descendant, depth)
    VALUES (NEW.as_id, NEW.as_id, 0);

    -- If parent exists, insert all ancestor paths
    IF NEW.as_in_reply_to IS NOT NULL THEN
        INSERT INTO objects_closure (ancestor, descendant, depth)
        SELECT p.ancestor, NEW.as_id, p.depth + 1
        FROM objects_closure p
        WHERE p.descendant = NEW.as_in_reply_to;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 2. Trigger function for UPDATE (if parent changes)
CREATE OR REPLACE FUNCTION objects_closure_update_trigger() RETURNS TRIGGER AS $$
BEGIN
    -- Only process if parent changed
    IF NEW.as_in_reply_to IS DISTINCT FROM OLD.as_in_reply_to THEN
        -- Delete old closure paths
        DELETE FROM objects_closure
        WHERE descendant = NEW.as_id AND depth > 0;
        
        -- Insert new paths if new parent exists
        IF NEW.as_in_reply_to IS NOT NULL THEN
            INSERT INTO objects_closure (ancestor, descendant, depth)
            SELECT p.ancestor, NEW.as_id, p.depth + 1
            FROM objects_closure p
            WHERE p.descendant = NEW.as_in_reply_to;
        END IF;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 3. Trigger function for DELETE (handled by CASCADE)

-- Create triggers
CREATE TRIGGER trg_objects_closure_insert
AFTER INSERT ON objects
FOR EACH ROW
EXECUTE FUNCTION objects_closure_insert_trigger();

CREATE TRIGGER trg_objects_closure_update
AFTER UPDATE OF as_in_reply_to ON objects
FOR EACH ROW
WHEN (NEW.as_in_reply_to IS DISTINCT FROM OLD.as_in_reply_to)
EXECUTE FUNCTION objects_closure_update_trigger();
