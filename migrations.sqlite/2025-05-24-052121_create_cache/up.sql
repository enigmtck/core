CREATE TABLE cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    uuid TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL UNIQUE,
    media_type TEXT,
    height INTEGER,
    width INTEGER,
    blurhash TEXT,
    path TEXT
);

CREATE INDEX idx_cache_url ON cache(url);

CREATE TRIGGER cache_auto_update_updated_at
AFTER UPDATE ON cache
FOR EACH ROW
BEGIN
    UPDATE cache
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
