-- This reverts the trigger function to its previous version, which handles
-- JSONB `as_in_reply_to` values but does not handle adopting orphaned children.
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
