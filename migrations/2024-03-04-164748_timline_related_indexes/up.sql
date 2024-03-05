CREATE INDEX idx_activities_to_activity_id ON activities_to USING btree (activity_id);
CREATE INDEX idx_activities_to_ap_id ON activities_to USING btree (ap_id);
CREATE INDEX idx_activities_cc_activity_id ON activities_cc USING btree (activity_id);
CREATE INDEX idx_activities_cc_ap_id ON activities_cc USING btree (ap_id);
CREATE INDEX idx_timeline_to_timeline_id ON timeline_to USING btree (timeline_id);
CREATE INDEX idx_timeline_to_ap_id ON timeline_to USING btree (ap_id);
CREATE INDEX idx_timeline_cc_timeline_id ON timeline_cc USING btree (timeline_id);
CREATE INDEX idx_timeline_hashtags_timeline_id ON timeline_hashtags USING btree (timeline_id);
