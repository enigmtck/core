CREATE TABLE instances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    domain_name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    "json" BLOB,
    blocked INTEGER NOT NULL DEFAULT 0,
    last_message_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    shared_inbox TEXT
);

CREATE TRIGGER instances_auto_update_updated_at
AFTER UPDATE ON instances
FOR EACH ROW
BEGIN
    UPDATE instances
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
