CREATE TABLE cache (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    uuid TEXT NOT NULL,
    url TEXT NOT NULL,
    media_type TEXT,
    height INTEGER,
    width INTEGER,
    blurhash TEXT
);

CREATE TRIGGER cache_updated_at
    AFTER UPDATE ON cache FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE cache SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE INDEX idx_cache_url ON cache (url);

