CREATE TABLE activities (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    kind TEXT CHECK(kind IN ('create','delete','update','announce','like','undo','follow','accept','block','add','remove')) NOT NULL,
    uuid TEXT NOT NULL,
    actor TEXT NOT NULL COLLATE NOCASE,
    ap_to JSONB,
    cc JSONB,
    target_activity_id INTEGER,
    target_ap_id TEXT COLLATE NOCASE,
    revoked BOOLEAN DEFAULT 0 NOT NULL,
    ap_id TEXT COLLATE NOCASE,
    reply BOOLEAN DEFAULT 0 NOT NULL,
    raw JSONB,
    target_object_id INTEGER,
    actor_id INTEGER,
    target_actor_id INTEGER,
    log JSONB,
    instrument JSONB
);

CREATE TRIGGER activities_updated_at
    AFTER UPDATE ON activities FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE activities SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE INDEX activities_idx_revoked_target_id ON activities (revoked, target_ap_id);
CREATE INDEX idx_activities_actor ON activities (actor);
CREATE INDEX idx_activities_actor_id ON activities (actor_id);
CREATE INDEX idx_ap_to ON activities (json_extract(ap_to, '$[*]'));
CREATE INDEX idx_cc ON activities (json_extract(cc, '$[*]'));
CREATE INDEX idx_activities_actor_created_at ON activities (created_at);
CREATE INDEX idx_activities_actor_created_at_desc ON activities (created_at DESC);
CREATE INDEX idx_activities_kind ON activities (kind);

