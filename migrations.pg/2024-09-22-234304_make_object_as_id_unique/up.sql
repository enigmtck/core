DELETE FROM
    objects a
        USING objects b
WHERE
    a.created_at < b.created_at
    AND a.as_id = b.as_id;

ALTER TABLE objects ADD CONSTRAINT uniq_objects_as_id UNIQUE (as_id);
