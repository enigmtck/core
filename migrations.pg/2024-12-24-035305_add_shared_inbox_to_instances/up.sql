ALTER TABLE instances ADD COLUMN shared_inbox TEXT;
WITH first_matches AS (
    SELECT DISTINCT ON (i.id)
        i.id,
        jsonb_extract_path_text(a.as_endpoints, 'sharedInbox') as shared_inbox
    FROM instances i
    JOIN actors a ON split_part(a.ek_webfinger COLLATE "C", '@', 3) = i.domain_name
)
UPDATE instances i
SET shared_inbox = fm.shared_inbox
FROM first_matches fm
WHERE i.id = fm.id;
