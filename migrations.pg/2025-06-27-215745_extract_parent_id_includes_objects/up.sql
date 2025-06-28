-- Update extract_parent_id function to handle JSON objects with "id" key
CREATE OR REPLACE FUNCTION extract_parent_id(parent_ref JSONB) RETURNS TEXT AS $$
BEGIN
    RETURN CASE 
        WHEN jsonb_typeof(parent_ref) = 'string' THEN parent_ref #>> '{}'
        WHEN jsonb_typeof(parent_ref) = 'array' AND jsonb_array_length(parent_ref) > 0 THEN
            CASE 
                WHEN jsonb_typeof(parent_ref -> 0) = 'string' THEN (parent_ref -> 0) #>> '{}'
                WHEN jsonb_typeof(parent_ref -> 0) = 'object' AND (parent_ref -> 0) ? 'id' 
                    THEN (parent_ref -> 0) ->> 'id'
                ELSE NULL
            END
        WHEN jsonb_typeof(parent_ref) = 'object' AND parent_ref ? 'id' 
            THEN parent_ref ->> 'id'
        ELSE NULL
    END;
END;
$$ LANGUAGE plpgsql IMMUTABLE;
