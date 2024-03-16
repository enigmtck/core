ALTER TABLE activities ADD CONSTRAINT fk_profile_activities FOREIGN KEY (profile_id) REFERENCES profiles(id);
