ALTER TABLE leaders ALTER COLUMN actor SET DATA TYPE VARCHAR COLLATE "case_insensitive";
ALTER TABLE leaders ALTER COLUMN leader_ap_id SET DATA TYPE VARCHAR COLLATE "case_insensitive";
ALTER TABLE leaders ALTER COLUMN accept_ap_id SET DATA TYPE VARCHAR COLLATE "case_insensitive";
ALTER TABLE leaders ALTER COLUMN follow_ap_id SET DATA TYPE VARCHAR COLLATE "case_insensitive";
ALTER TABLE followers ALTER COLUMN actor SET DATA TYPE VARCHAR COLLATE "case_insensitive";
ALTER TABLE followers ALTER COLUMN followed_ap_id SET DATA TYPE VARCHAR COLLATE "case_insensitive";
ALTER TABLE followers ALTER COLUMN ap_id SET DATA TYPE VARCHAR COLLATE "case_insensitive";

