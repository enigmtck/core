CREATE TABLE olm_sessions (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    uuid TEXT NOT NULL,
    session_data TEXT NOT NULL,
    session_hash TEXT NOT NULL,
    owner_as_id TEXT NOT NULL COLLATE NOCASE,
    ap_conversation TEXT NOT NULL,
    owner_id INTEGER NOT NULL,
    FOREIGN KEY (owner_as_id) REFERENCES actors(as_id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id) REFERENCES actors(id)
);

CREATE TRIGGER olm_sessions_updated_at
    AFTER UPDATE ON olm_sessions FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE olm_sessions SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE UNIQUE INDEX uniq_olm_sessions_uuid ON olm_sessions (uuid);
CREATE UNIQUE INDEX uniq_olm_sessions_owner_conversation ON olm_sessions (ap_conversation, owner_as_id);

CREATE INDEX olm_sessions_owner_as_id ON olm_sessions (owner_as_id);
CREATE INDEX olm_sessions_uuid ON olm_sessions (uuid);
