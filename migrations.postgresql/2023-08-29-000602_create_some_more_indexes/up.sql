CREATE INDEX idx_activities_kind ON activities USING btree (kind);
CREATE INDEX idx_activities_target_ap_id ON activities USING btree (target_ap_id);
CREATE INDEX idx_activities_profile_id ON activities USING btree (profile_id);
