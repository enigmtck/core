CREATE TABLE encrypted_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    profile_id INTEGER NOT NULL, -- Assuming FK to actors(id)
    ap_to TEXT NOT NULL,
    attributed_to TEXT NOT NULL,
    instrument BLOB NOT NULL,
    reference TEXT,
    uuid TEXT NOT NULL,
    FOREIGN KEY(profile_id) REFERENCES actors(id) -- Added based on common patterns, verify if intended
);

CREATE TRIGGER encrypted_sessions_auto_update_updated_at
AFTER UPDATE ON encrypted_sessions
FOR EACH ROW
BEGIN
    UPDATE encrypted_sessions
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
