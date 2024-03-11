ALTER TABLE remote_activities DROP CONSTRAINT uniq_remote_activities_ap_id;
ALTER TABLE remote_activities ADD COLUMN profile_id i32;
