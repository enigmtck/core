CREATE TABLE unprocessable (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    raw BLOB NOT NULL,
    error TEXT
);

CREATE TRIGGER unprocessable_auto_update_updated_at
AFTER UPDATE ON unprocessable
FOR EACH ROW
BEGIN
    UPDATE unprocessable
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
