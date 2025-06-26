-- Step 1: Drop triggers created in this migration
DROP TRIGGER IF EXISTS trg_objects_closure_insert ON objects;
DROP TRIGGER IF EXISTS trg_objects_closure_update ON objects;

-- Step 2: Drop functions created in this migration
DROP FUNCTION IF EXISTS objects_closure_insert_trigger() CASCADE;
DROP FUNCTION IF EXISTS objects_closure_update_trigger() CASCADE;
DROP FUNCTION IF EXISTS extract_parent_id(JSONB) CASCADE;

-- Step 3: Drop foreign key constraint added in this migration
ALTER TABLE objects DROP CONSTRAINT IF EXISTS fk_objects_parent;

-- Step 4: Convert column back to TEXT
ALTER TABLE objects 
ALTER COLUMN as_in_reply_to TYPE TEXT 
USING CASE 
    WHEN as_in_reply_to IS NULL THEN NULL 
    ELSE as_in_reply_to #>> '{}' 
END;

-- Step 5: Recreate helper function for TEXT
CREATE OR REPLACE FUNCTION extract_parent_id(parent_ref TEXT) RETURNS TEXT AS $$
BEGIN
    RETURN parent_ref;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Step 6: Recreate trigger functions for TEXT
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

-- Step 7: Recreate triggers
CREATE TRIGGER trg_objects_closure_insert
AFTER INSERT ON objects
FOR EACH ROW
EXECUTE FUNCTION objects_closure_insert_trigger();

CREATE TRIGGER trg_objects_closure_update
AFTER UPDATE OF as_in_reply_to ON objects
FOR EACH ROW
WHEN (NEW.as_in_reply_to IS DISTINCT FROM OLD.as_in_reply_to)
EXECUTE FUNCTION objects_closure_update_trigger();

-- Step 8: Re-add foreign key constraint (if it existed previously)
ALTER TABLE objects
ADD CONSTRAINT fk_objects_parent 
FOREIGN KEY (as_in_reply_to) 
REFERENCES objects(as_id) 
ON DELETE SET NULL;
