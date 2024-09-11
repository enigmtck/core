DROP INDEX idx_activities_created_at;
DROP INDEX idx_activities_revoked_created_at;
ALTER TABLE activities DROP COLUMN reply;
