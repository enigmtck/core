-- Convert featured from TEXT to JSONB in place
ALTER TABLE actors 
    ALTER COLUMN as_featured TYPE JSONB USING (
        CASE 
            WHEN as_featured IS NULL THEN NULL
            ELSE to_jsonb(as_featured::text)
        END
    );
