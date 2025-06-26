-- Drop triggers first to avoid function dependency issues
DROP TRIGGER IF EXISTS trg_objects_closure_insert ON objects;
DROP TRIGGER IF EXISTS trg_objects_closure_update ON objects;

-- Drop trigger functions with CASCADE to remove dependencies
DROP FUNCTION IF EXISTS objects_closure_insert_trigger() CASCADE;
DROP FUNCTION IF EXISTS objects_closure_update_trigger() CASCADE;

-- Drop foreign key constraint on objects table
ALTER TABLE objects DROP CONSTRAINT IF EXISTS fk_objects_parent;

-- Drop closure table and all its dependencies
DROP TABLE IF EXISTS objects_closure CASCADE;
