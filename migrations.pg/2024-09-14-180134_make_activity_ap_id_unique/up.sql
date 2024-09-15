DELETE FROM activities a USING activities b WHERE a.id < b.id AND a.ap_id = b.ap_id;
ALTER TABLE activities ADD CONSTRAINT uniq_activities_ap_id UNIQUE (ap_id);
