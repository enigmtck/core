CREATE TABLE processing_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    profile_id INTEGER NOT NULL, -- Assuming FK to actors(id)
    ap_id TEXT NOT NULL,
    ap_to BLOB NOT NULL,
    cc BLOB,
    attributed_to TEXT NOT NULL,
    kind TEXT NOT NULL,
    ap_object BLOB NOT NULL,
    processed INTEGER NOT NULL, -- Boolean
    FOREIGN KEY(profile_id) REFERENCES actors(id) -- Added based on common patterns
);

CREATE TRIGGER processing_queue_auto_update_updated_at
AFTER UPDATE ON processing_queue
FOR EACH ROW
BEGIN
    UPDATE processing_queue
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
