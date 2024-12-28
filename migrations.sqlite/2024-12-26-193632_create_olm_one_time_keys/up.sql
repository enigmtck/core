CREATE TABLE olm_one_time_keys (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    uuid TEXT NOT NULL,
    profile_id INTEGER NOT NULL,
    olm_id INTEGER NOT NULL,
    key_data TEXT NOT NULL,
    distributed BOOLEAN DEFAULT 0 NOT NULL,
    assignee TEXT,
    FOREIGN KEY (profile_id) REFERENCES actors(id) ON DELETE CASCADE
);

CREATE TRIGGER olm_one_time_keys_updated_at
    AFTER UPDATE ON olm_one_time_keys FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE olm_one_time_keys SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

