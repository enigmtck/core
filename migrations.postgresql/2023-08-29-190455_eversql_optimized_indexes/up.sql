CREATE INDEX activities_idx_revoked_target_id ON "activities" ("revoked","target_ap_id");
CREATE INDEX timeline_idx_ap_public_created_at ON "timeline" ("ap_public","created_at" desc);
CREATE INDEX timeline_idx_created_at ON "timeline" ("created_at" desc);
