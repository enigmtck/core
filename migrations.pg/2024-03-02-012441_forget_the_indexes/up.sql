CREATE INDEX idx_remote_note_hashtags_hashtag ON remote_note_hashtags USING btree (hashtag);
CREATE INDEX idx_note_hashtags_hashtag ON note_hashtags USING btree (hashtag);
CREATE INDEX idx_remote_actor_hashtags_hashtag ON remote_actor_hashtags USING btree (hashtag);
CREATE INDEX idx_profile_hashtags_hashtag ON profile_hashtags USING btree (hashtag);
CREATE INDEX idx_timeline_hashtags_hashtag ON timeline_hashtags USING btree (hashtag);
