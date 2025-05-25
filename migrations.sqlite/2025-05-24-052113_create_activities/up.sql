CREATE TABLE activities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    kind TEXT NOT NULL CHECK (kind IN ('create', 'delete', 'update', 'announce', 'like', 'undo', 'follow', 'accept', 'block', 'add', 'remove')),
    uuid TEXT NOT NULL,
    actor TEXT NOT NULL COLLATE NOCASE,
    ap_to BLOB,
    cc BLOB,
    target_activity_id INTEGER,
    target_ap_id TEXT COLLATE NOCASE,
    revoked INTEGER NOT NULL DEFAULT 0,
    ap_id TEXT COLLATE NOCASE UNIQUE,
    reply INTEGER NOT NULL DEFAULT 0,
    raw BLOB,
    target_object_id INTEGER,
    actor_id INTEGER,
    target_actor_id INTEGER,
    log BLOB,
    instrument BLOB,
    FOREIGN KEY(actor_id) REFERENCES actors(id) ON DELETE CASCADE,
    FOREIGN KEY(target_actor_id) REFERENCES actors(id)
);

CREATE INDEX idx_activities_actor ON activities(actor);
CREATE INDEX idx_activities_actor_id ON activities(actor_id);
CREATE INDEX idx_activities_ap_to ON activities(ap_to); -- Note: Indexing BLOB/JSONB might behave differently or not be as effective as in PG.
CREATE INDEX idx_activities_cc ON activities(cc); -- Note: Indexing BLOB/JSONB might behave differently or not be as effective as in PG.
CREATE INDEX idx_activities_composite ON activities(revoked, reply, kind) WHERE (revoked = 0 AND reply = 0);
CREATE INDEX idx_activities_created_at ON activities(created_at);
CREATE INDEX idx_activities_created_at_desc ON activities(created_at DESC);
CREATE INDEX idx_activities_kind ON activities(kind);
CREATE INDEX idx_activities_kind_created_at ON activities(kind, created_at);
CREATE INDEX idx_activities_revoked_created_at ON activities(revoked, created_at);
CREATE INDEX activities_idx_revoked_target_id ON activities(revoked, target_ap_id);
CREATE INDEX idx_activities_target_actor_id ON activities(target_actor_id);
CREATE INDEX idx_activities_target_ap_id ON activities(target_ap_id);
CREATE INDEX idx_activities_target_object_id ON activities(target_object_id);
-- idx_activities_target is more complex, might need adjustment or simplification for SQLite if it involves OR logic in WHERE.
-- PG: WHERE ((target_object_id IS NOT NULL) OR (target_activity_id IS NOT NULL))
-- SQLite partial index WHERE clause needs to be a simple expression.
-- For now, let's create separate indexes or a general one if specific logic is hard to translate.
-- Let's assume separate indexes are fine or this specific complex partial index is omitted for simplicity unless critical.
-- CREATE INDEX idx_activities_target_object_id_not_null ON activities(target_object_id) WHERE target_object_id IS NOT NULL;
-- CREATE INDEX idx_activities_target_activity_id_not_null ON activities(target_activity_id) WHERE target_activity_id IS NOT NULL;


CREATE TRIGGER activities_auto_update_updated_at
AFTER UPDATE ON activities
FOR EACH ROW
BEGIN
    UPDATE activities
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
