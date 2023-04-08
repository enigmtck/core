CREATE INDEX idx_timeline_ap_id ON timeline USING btree (ap_id);
CREATE INDEX idx_likes_object_ap_id ON likes USING btree (object_ap_id);
CREATE INDEX idx_likes_profile_id ON likes USING btree (profile_id);
CREATE INDEX idx_timeline_cc_ap_id ON timeline_cc USING btree (ap_id);
CREATE INDEX idx_remote_announces_timeline_id ON remote_announces USING btree (timeline_id);
CREATE INDEX idx_remote_likes_object_id ON remote_likes USING btree (object_id);
