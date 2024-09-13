CREATE INDEX idx_activities_ap_to ON activities USING gin (ap_to);
CREATE INDEX idx_activities_created_at_desc ON activities USING btree (created_at desc);
CREATE INDEX idx_activities_kind_created_at ON activities USING btree (kind, created_at);
CREATE INDEX idx_activities_target_note_id ON activities USING btree (target_note_id);
CREATE INDEX idx_activities_target_remote_note_id ON activities USING btree (target_remote_note_id);

UPDATE activities SET reply = false;

UPDATE activities
   SET reply = true
WHERE
  target_remote_note_id IN (
    SELECT r.id FROM activities a
      LEFT JOIN remote_notes r ON (a.target_remote_note_id = r.id)
      WHERE r.in_reply_to IS NOT NULL
  );

UPDATE activities
   SET reply = true
WHERE
  target_note_id IN (
    SELECT n.id FROM activities a
      LEFT JOIN notes n ON (a.target_note_id = n.id)
      WHERE n.in_reply_to IS NOT NULL
  );
