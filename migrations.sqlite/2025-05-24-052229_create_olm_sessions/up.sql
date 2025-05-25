CREATE TABLE olm_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    uuid TEXT NOT NULL UNIQUE,
    session_data TEXT NOT NULL,
    session_hash TEXT NOT NULL,
    owner_as_id TEXT NOT NULL COLLATE NOCASE,
    ap_conversation TEXT NOT NULL,
    owner_id INTEGER NOT NULL,
    FOREIGN KEY(owner_as_id) REFERENCES actors(as_id) ON DELETE CASCADE,
    FOREIGN KEY(owner_id) REFERENCES actors(id),
    UNIQUE(ap_conversation, owner_as_id)
);

CREATE INDEX olm_sessions_owner_as_id ON olm_sessions(owner_as_id);
CREATE INDEX olm_sessions_uuid_idx ON olm_sessions(uuid); -- Renamed from olm_sessions_uuid to avoid conflict with unique constraint name

CREATE TRIGGER olm_sessions_auto_update_updated_at
AFTER UPDATE ON olm_sessions
FOR EACH ROW
BEGIN
    UPDATE olm_sessions
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
