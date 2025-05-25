CREATE TABLE vault (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    uuid TEXT NOT NULL UNIQUE,
    owner_as_id TEXT NOT NULL COLLATE NOCASE,
    activity_id INTEGER NOT NULL,
    data TEXT NOT NULL,
    FOREIGN KEY(owner_as_id) REFERENCES actors(as_id) ON DELETE CASCADE,
    FOREIGN KEY(activity_id) REFERENCES activities(id) ON DELETE CASCADE
);

CREATE INDEX vault_activity_id ON vault(activity_id);
CREATE INDEX vault_owner_as_id ON vault(owner_as_id);
CREATE INDEX vault_uuid_idx ON vault(uuid); -- Renamed from vault_uuid to avoid conflict with unique constraint name

CREATE TRIGGER vault_auto_update_updated_at
AFTER UPDATE ON vault
FOR EACH ROW
BEGIN
    UPDATE vault
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
