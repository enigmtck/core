CREATE TABLE remote_encrypted_sessions (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    profile_id INTEGER NOT NULL,
    actor TEXT NOT NULL COLLATE NOCASE,
    kind TEXT NOT NULL,
    ap_id TEXT NOT NULL COLLATE NOCASE UNIQUE,
    ap_to TEXT NOT NULL,
    attributed_to TEXT NOT NULL COLLATE NOCASE,
    instrument TEXT NOT NULL,
    reference TEXT,
    FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

CREATE TRIGGER remote_encrypted_sessions_updated_at
    AFTER UPDATE ON remote_encrypted_sessions FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE remote_encrypted_sessions SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
