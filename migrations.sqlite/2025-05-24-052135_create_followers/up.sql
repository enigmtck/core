CREATE TABLE followers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    ap_id TEXT NOT NULL UNIQUE COLLATE NOCASE,
    actor TEXT NOT NULL COLLATE NOCASE,
    followed_ap_id TEXT NOT NULL COLLATE NOCASE,
    uuid TEXT NOT NULL UNIQUE,
    actor_id INTEGER NOT NULL DEFAULT 0, -- Assuming FK to actors(id)
    UNIQUE(actor, followed_ap_id),
    FOREIGN KEY(actor_id) REFERENCES actors(id) -- Added based on common patterns, verify if intended
);

CREATE INDEX idx_followers_actor_id ON followers(actor_id);

CREATE TRIGGER followers_auto_update_updated_at
AFTER UPDATE ON followers
FOR EACH ROW
BEGIN
    UPDATE followers
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
