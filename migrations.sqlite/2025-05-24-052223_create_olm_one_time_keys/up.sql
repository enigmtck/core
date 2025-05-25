CREATE TABLE olm_one_time_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    uuid TEXT NOT NULL,
    profile_id INTEGER NOT NULL,
    olm_id INTEGER NOT NULL, -- This was not an FK in PG dump, but name suggests it might be.
    key_data TEXT NOT NULL,
    distributed INTEGER NOT NULL DEFAULT 0,
    assignee TEXT,
    FOREIGN KEY(profile_id) REFERENCES actors(id) ON DELETE CASCADE
);

CREATE TRIGGER olm_one_time_keys_auto_update_updated_at
AFTER UPDATE ON olm_one_time_keys
FOR EACH ROW
BEGIN
    UPDATE olm_one_time_keys
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
