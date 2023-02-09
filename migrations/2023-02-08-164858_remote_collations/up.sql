ALTER TABLE remote_actors ALTER COLUMN ap_id SET DATA TYPE VARCHAR COLLATE "case_insensitive";
ALTER TABLE remote_activities ALTER COLUMN ap_id SET DATA TYPE VARCHAR COLLATE "case_insensitive";
ALTER TABLE remote_notes ALTER COLUMN ap_id SET DATA TYPE VARCHAR COLLATE "case_insensitive";
