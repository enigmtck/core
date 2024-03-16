ALTER TABLE activities DROP CONSTRAINT fk_profile_activities;
ALTER TABLE activities ALTER COLUMN profile_id DROP NOT NULL;
