-- Revert extract_parent_id function to previous version
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
