-- Optimize the closure table trigger to fix slow query planning
--
-- The issue was that using NEW.as_id directly in queries caused PostgreSQL's
-- query planner to create suboptimal plans, particularly for the orphan-finding
-- queries which would take 1.5+ seconds even when no orphans existed.
--
-- By copying NEW.as_id to a local variable first, PostgreSQL creates much better
-- query plans that properly use indexes, reducing execution time from 1.5s to <20ms.

CREATE OR REPLACE FUNCTION objects_closure_insert_trigger()
  RETURNS TRIGGER
  AS $$
  DECLARE
      parent_id TEXT;
      new_object_id TEXT;
  BEGIN
      -- Copy NEW.as_id to local variable for better query planning
      new_object_id := NEW.as_id;

      -- Insert self-reference for the new object (depth 0).
      INSERT INTO objects_closure (ancestor, descendant, depth)
      VALUES (new_object_id, new_object_id, 0)
      ON CONFLICT (ancestor, descendant) DO NOTHING;

      -- If the new object has a parent, connect it to its ancestors.
      parent_id := extract_parent_id(NEW.as_in_reply_to);

      IF parent_id IS NOT NULL THEN
          INSERT INTO objects_closure (ancestor, descendant, depth)
          SELECT p.ancestor, new_object_id, p.depth + 1
          FROM objects_closure p
          WHERE p.descendant = parent_id
          ON CONFLICT (ancestor, descendant) DO NOTHING;
      END IF;

      -- Find any existing children that were orphaned and connect them.
      IF EXISTS (
          SELECT 1
          FROM objects o
          WHERE extract_parent_id(o.as_in_reply_to) = new_object_id
          LIMIT 1
      ) THEN
          INSERT INTO objects_closure (ancestor, descendant, depth)
          SELECT
              p.ancestor,
              c.descendant,
              p.depth + c.depth + 1
          FROM objects_closure p
          JOIN objects o ON extract_parent_id(o.as_in_reply_to) = new_object_id
          JOIN objects_closure c ON c.ancestor = o.as_id
          WHERE p.descendant = new_object_id
          ON CONFLICT (ancestor, descendant) DO NOTHING;
      END IF;

      RETURN NEW;
  END;
  $$ LANGUAGE plpgsql;
