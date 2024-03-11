DELETE FROM timeline_cc t1
USING timeline_cc t2
WHERE t1.id > t2.id
    AND t1.timeline_id = t2.timeline_id
    AND t1.ap_id = t2.ap_id;

ALTER TABLE timeline_cc ADD CONSTRAINT uniq_timeline_cc_id_ap_id UNIQUE (timeline_id, ap_id);

DELETE FROM timeline_to t1
USING timeline_to t2
WHERE t1.id > t2.id
    AND t1.timeline_id = t2.timeline_id
    AND t1.ap_id = t2.ap_id;

ALTER TABLE timeline_to ADD CONSTRAINT uniq_timeline_to_id_ap_id UNIQUE (timeline_id, ap_id);
