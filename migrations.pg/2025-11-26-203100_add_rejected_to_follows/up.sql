ALTER TABLE follows ADD COLUMN rejected BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE follows ADD COLUMN reject_activity_ap_id TEXT;
