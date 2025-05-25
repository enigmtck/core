CREATE TABLE mls_key_packages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    uuid TEXT NOT NULL,
    actor_id INTEGER NOT NULL, -- Assuming FK to actors(id)
    key_data TEXT NOT NULL,
    distributed INTEGER NOT NULL DEFAULT 0,
    assignee TEXT,
    FOREIGN KEY(actor_id) REFERENCES actors(id) -- Added based on common patterns, verify if intended
);

CREATE INDEX idx_actor_id ON mls_key_packages(actor_id); -- Renamed from PG's idx_actor_id for clarity if it was generic
CREATE INDEX idx_mls_key_packages_created_at_asc ON mls_key_packages(created_at); -- Renamed from idx_created_at_asc
CREATE INDEX idx_mls_key_packages_uuid ON mls_key_packages(uuid); -- Renamed from idx_uuid

CREATE TRIGGER mls_key_packages_auto_update_updated_at
AFTER UPDATE ON mls_key_packages
FOR EACH ROW
BEGIN
    UPDATE mls_key_packages
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
