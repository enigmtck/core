CREATE TABLE follows (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    uuid TEXT NOT NULL,
    profile_id INTEGER, -- Assuming FK to actors(id)
    ap_object TEXT NOT NULL,
    actor TEXT NOT NULL,
    FOREIGN KEY(profile_id) REFERENCES actors(id) -- Added based on common patterns, verify if intended
);

CREATE TRIGGER follows_auto_update_updated_at
AFTER UPDATE ON follows
FOR EACH ROW
BEGIN
    UPDATE follows
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
