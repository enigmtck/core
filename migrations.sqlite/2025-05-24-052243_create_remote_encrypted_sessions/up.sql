CREATE TABLE remote_encrypted_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    profile_id INTEGER NOT NULL, -- Assuming FK to actors(id)
    actor TEXT NOT NULL,
    kind TEXT NOT NULL,
    ap_id TEXT NOT NULL UNIQUE,
    ap_to TEXT NOT NULL,
    attributed_to TEXT NOT NULL,
    instrument BLOB NOT NULL,
    reference TEXT,
    FOREIGN KEY(profile_id) REFERENCES actors(id) -- Added based on common patterns
);

CREATE TRIGGER remote_encrypted_sessions_auto_update_updated_at
AFTER UPDATE ON remote_encrypted_sessions
FOR EACH ROW
BEGIN
    UPDATE remote_encrypted_sessions
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
