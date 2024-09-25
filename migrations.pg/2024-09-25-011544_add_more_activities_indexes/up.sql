CREATE INDEX idx_activities_cc ON activities USING gin (cc);
CREATE INDEX idx_activities_target_object_id ON activities (target_object_id);

