-- Convert featured from JSONB back to TEXT in place
ALTER TABLE actors 
    ALTER COLUMN as_featured TYPE TEXT USING (
        CASE 
            WHEN as_featured IS NULL THEN NULL
            WHEN jsonb_typeof(as_featured) = 'string' THEN as_featured #>> '{}'
            ELSE as_featured::text
        END
    );
