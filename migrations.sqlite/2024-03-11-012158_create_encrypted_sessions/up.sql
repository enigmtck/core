CREATE TABLE encrypted_sessions (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    profile_id INTEGER NOT NULL,
    ap_to TEXT NOT NULL,
    attributed_to TEXT NOT NULL,
    instrument TEXT NOT NULL,
    reference TEXT,
    uuid TEXT NOT NULL,
    FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

CREATE TRIGGER encrypted_sessions_updated_at
    AFTER UPDATE ON encrypted_sessions FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE encrypted_sessions SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
