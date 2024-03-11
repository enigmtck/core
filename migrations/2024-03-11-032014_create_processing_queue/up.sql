CREATE TABLE processing_queue (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    ap_id TEXT NOT NULL COLLATE NOCASE,
    ap_to TEXT NOT NULL,
    cc TEXT NOT NULL,
    attributed_to TEXT NOT NULL COLLATE NOCASE,
    kind TEXT NOT NULL,
    ap_object TEXT NOT NULL,
    processed BOOLEAN NOT NULL DEFAULT 0,
    profile_id INTEGER NOT NULL
);

CREATE TRIGGER processing_queue_updated_at
    AFTER UPDATE ON processing_queue FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE processing_queue SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
