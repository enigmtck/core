DELETE FROM timeline t1
USING timeline t2
WHERE t1.id > t2.id
    AND t1.ap_id = t2.ap_id;

ALTER TABLE timeline ADD CONSTRAINT uniq_timeline_ap_id UNIQUE (ap_id);
