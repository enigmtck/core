ALTER TABLE remote_activities ADD CONSTRAINT uniq_remote_activities_ap_id UNIQUE (ap_id);
ALTER TABLE remote_activities DROP COLUMN profile_id;
