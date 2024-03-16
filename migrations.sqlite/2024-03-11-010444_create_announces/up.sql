CREATE TABLE announces (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    profile_id INTEGER,
    uuid TEXT NOT NULL,
    actor TEXT NOT NULL COLLATE NOCASE,
    ap_to TEXT NOT NULL,
    cc TEXT,
    object_ap_id TEXT NOT NULL COLLATE NOCASE
);

CREATE TRIGGER announces_updated_at
    AFTER UPDATE ON activities FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE announces SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
