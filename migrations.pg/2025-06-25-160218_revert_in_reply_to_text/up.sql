-- Step 1: Drop triggers and functions
DROP TRIGGER IF EXISTS trg_objects_closure_insert ON objects;
DROP TRIGGER IF EXISTS trg_objects_closure_update ON objects;
DROP FUNCTION IF EXISTS objects_closure_insert_trigger() CASCADE;
DROP FUNCTION IF EXISTS objects_closure_update_trigger() CASCADE;

-- Step 2: Drop foreign key constraint
ALTER TABLE objects DROP CONSTRAINT IF EXISTS fk_objects_parent;

-- Step 3: Convert column back to JSONB
ALTER TABLE objects 
ALTER COLUMN as_in_reply_to TYPE JSONB 
USING CASE 
    WHEN as_in_reply_to IS NULL THEN NULL 
    ELSE to_jsonb(as_in_reply_to) 
END;

-- Step 4: Create helper function to extract parent ID
CREATE OR REPLACE FUNCTION extract_parent_id(parent_ref JSONB) RETURNS TEXT AS $$
BEGIN
    RETURN CASE 
        WHEN jsonb_typeof(parent_ref) = 'string' THEN parent_ref #>> '{}'
        WHEN jsonb_typeof(parent_ref) = 'array' AND jsonb_array_length(parent_ref) > 0 
            THEN (parent_ref -> 0) #>> '{}'
        ELSE NULL
    END;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Step 5: Update trigger functions for JSONB
CREATE OR REPLACE FUNCTION objects_closure_insert_trigger() RETURNS TRIGGER AS $$
DECLARE
    parent_id TEXT;
BEGIN
    INSERT INTO objects_closure (ancestor, descendant, depth)
    VALUES (NEW.as_id, NEW.as_id, 0);

    parent_id := extract_parent_id(NEW.as_in_reply_to);
    
    IF parent_id IS NOT NULL THEN
        INSERT INTO objects_closure (ancestor, descendant, depth)
        SELECT p.ancestor, NEW.as_id, p.depth + 1
        FROM objects_closure p
        WHERE p.descendant = parent_id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION objects_closure_update_trigger() RETURNS TRIGGER AS $$
DECLARE
    old_parent_id TEXT;
    new_parent_id TEXT;
BEGIN
    old_parent_id := extract_parent_id(OLD.as_in_reply_to);
    new_parent_id := extract_parent_id(NEW.as_in_reply_to);
    
    IF old_parent_id IS DISTINCT FROM new_parent_id THEN
        DELETE FROM objects_closure
        WHERE descendant = NEW.as_id AND depth > 0;
        
        IF new_parent_id IS NOT NULL THEN
            INSERT INTO objects_closure (ancestor, descendant, depth)
            SELECT p.ancestor, NEW.as_id, p.depth + 1
            FROM objects_closure p
            WHERE p.descendant = new_parent_id;
        END IF;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Step 6: Recreate triggers
CREATE TRIGGER trg_objects_closure_insert
AFTER INSERT ON objects
FOR EACH ROW
EXECUTE FUNCTION objects_closure_insert_trigger();

CREATE TRIGGER trg_objects_closure_update
AFTER UPDATE OF as_in_reply_to ON objects
FOR EACH ROW
WHEN (NEW.as_in_reply_to IS DISTINCT FROM OLD.as_in_reply_to)
EXECUTE FUNCTION objects_closure_update_trigger();
