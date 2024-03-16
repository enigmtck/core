CREATE TABLE olm_sessions (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    uuid TEXT NOT NULL,
    session_data TEXT NOT NULL,
    session_hash TEXT NOT NULL,
    encrypted_session_id INTEGER NOT NULL,
    FOREIGN KEY(encrypted_session_id) REFERENCES encrypted_sessions(id)
);

CREATE TRIGGER olm_sessions_updated_at
    AFTER UPDATE ON olm_sessions FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE olm_sessions SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
