-- This migration updates the insert trigger to handle re-parenting of orphans.
-- When a parent object is inserted, this trigger will now find any existing
-- children that were previously orphaned and correctly establish their
-- hierarchical relationship in the closure table.
-- It is based on the version of the trigger that uses JSONB for `as_in_reply_to`.
-- CREATE OR REPLACE FUNCTION objects_closure_insert_trigger() RETURNS TRIGGER AS $$
-- DECLARE
--     parent_id TEXT;
-- BEGIN
--     -- Insert self-reference for the new object (depth 0).
--     INSERT INTO objects_closure (ancestor, descendant, depth)
--     VALUES (NEW.as_id, NEW.as_id, 0)
--     ON CONFLICT (ancestor, descendant) DO NOTHING;

--     -- If the new object has a parent, connect it to its ancestors.
--     parent_id := extract_parent_id(NEW.as_in_reply_to);
    
--     IF parent_id IS NOT NULL THEN
--         INSERT INTO objects_closure (ancestor, descendant, depth)
--         SELECT p.ancestor, NEW.as_id, p.depth + 1
--         FROM objects_closure p
--         WHERE p.descendant = parent_id
--         ON CONFLICT (ancestor, descendant) DO NOTHING;
--     END IF;

--     -- Find any existing children that were orphaned and connect them.
--     -- This query finds all objects `o` that are children of the `NEW` object,
--     -- and for each child, it connects all of `NEW`'s ancestors to all of the child's descendants.
--     INSERT INTO objects_closure (ancestor, descendant, depth)
--     SELECT
--         p.ancestor,      -- An ancestor of the new object (NEW)
--         c.descendant,    -- A descendant of an orphaned child
--         p.depth + c.depth + 1
--     FROM objects_closure p
    
--     -- Find children (o) of NEW by extracting their parent_id
--     JOIN objects o ON extract_parent_id(o.as_in_reply_to) = NEW.as_id
--     JOIN objects_closure c ON c.ancestor = o.as_id -- Find descendants of children
--     WHERE p.descendant = NEW.as_id
--     ON CONFLICT (ancestor, descendant) DO NOTHING;

--     RETURN NEW;
-- END;
-- $$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION objects_closure_insert_trigger()
  RETURNS TRIGGER
  AS $$
  DECLARE
      parent_id TEXT;
  BEGIN
      -- Insert self-reference for the new object (depth 0).
      INSERT INTO objects_closure (ancestor, descendant, depth)
      VALUES (NEW.as_id, NEW.as_id, 0)
      ON CONFLICT (ancestor, descendant) DO NOTHING;

      -- If the new object has a parent, connect it to its ancestors.
      parent_id := extract_parent_id(NEW.as_in_reply_to);

      IF parent_id IS NOT NULL THEN
          INSERT INTO objects_closure (ancestor, descendant, depth)
          SELECT p.ancestor, NEW.as_id, p.depth + 1
          FROM objects_closure p
          WHERE p.descendant = parent_id
          ON CONFLICT (ancestor, descendant) DO NOTHING;
      END IF;

      -- Find any existing children that were orphaned and connect them.
      IF EXISTS (
          SELECT 1
          FROM objects o
          WHERE extract_parent_id(o.as_in_reply_to) = NEW.as_id
          LIMIT 1
      ) THEN
          INSERT INTO objects_closure (ancestor, descendant, depth)
          SELECT
              p.ancestor,
              c.descendant,
              p.depth + c.depth + 1
          FROM objects_closure p
          JOIN objects o ON extract_parent_id(o.as_in_reply_to) = NEW.as_id
          JOIN objects_closure c ON c.ancestor = o.as_id
          WHERE p.descendant = NEW.as_id
          ON CONFLICT (ancestor, descendant) DO NOTHING;
      END IF;

      RETURN NEW;
  END;
  $$ LANGUAGE plpgsql;
