CREATE INDEX IF NOT EXISTS idx_activities_composite ON activities (revoked, reply, kind) 
  WHERE NOT revoked AND NOT reply;

CREATE INDEX IF NOT EXISTS idx_objects_type ON objects (as_type) 
  WHERE as_type != 'tombstone';

CREATE INDEX IF NOT EXISTS idx_activities_target 
  ON activities (target_object_id, target_activity_id)
  WHERE target_object_id IS NOT NULL OR target_activity_id IS NOT NULL;

