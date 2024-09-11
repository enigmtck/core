CREATE INDEX idx_activities_created_at ON activities USING btree (created_at);
CREATE INDEX idx_activities_revoked_created_at ON activities USING btree (revoked, created_at);
ALTER TABLE activities ADD COLUMN reply BOOLEAN NOT NULL DEFAULT false;

UPDATE activities
  SET reply = true
WHERE
  id IN (
    SELECT r.id FROM activities a
      LEFT JOIN remote_notes r ON (a.target_remote_note_id = r.id)
      WHERE r.in_reply_to IS NOT NULL
);
