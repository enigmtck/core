CREATE TABLE activities (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    profile_id INTEGER,
    kind TEXT CHECK(kind IN ('create','delete','update','announce','like','undo','follow','accept','block','add','remove')) NOT NULL,
    uuid TEXT NOT NULL,
    actor TEXT NOT NULL COLLATE NOCASE,
    ap_to TEXT,
    cc TEXT,
    target_note_id INTEGER,
    target_remote_note_id INTEGER,
    target_profile_id INTEGER,
    target_activity_id INTEGER,
    target_ap_id TEXT COLLATE NOCASE,
    target_remote_actor_id INTEGER,
    revoked BOOLEAN DEFAULT 0 NOT NULL,
    ap_id TEXT COLLATE NOCASE
);

CREATE TRIGGER activities_updated_at
    AFTER UPDATE ON activities FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE activities SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE INDEX activities_idx_revoked_target_id ON activities (revoked, target_ap_id);
CREATE INDEX idx_activities_kind ON activities (kind);
CREATE INDEX idx_activities_profile_id ON activities (profile_id);
CREATE INDEX idx_activities_target_ap_id ON activities (target_ap_id);

CREATE TABLE activities_cc (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    activity_id INTEGER NOT NULL,
    ap_id TEXT NOT NULL COLLATE NOCASE,
    FOREIGN KEY(activity_id) REFERENCES activities(id)
);

CREATE TRIGGER activities_cc_updated_at
    AFTER UPDATE ON activities FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE activities_cc SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE TABLE activities_to (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    activity_id INTEGER NOT NULL,
    ap_id TEXT NOT NULL COLLATE NOCASE,
    FOREIGN KEY(activity_id) REFERENCES activities(id)
);

CREATE TRIGGER activities_to_updated_at
    AFTER UPDATE ON activities FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE activities_to SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE INDEX idx_activities_cc_activity_id ON activities_cc (activity_id);
CREATE INDEX idx_activities_cc_ap_id ON activities_cc (ap_id);

