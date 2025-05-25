CREATE TABLE leaders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    actor TEXT NOT NULL COLLATE NOCASE,
    leader_ap_id TEXT NOT NULL COLLATE NOCASE,
    uuid TEXT NOT NULL UNIQUE,
    accept_ap_id TEXT COLLATE NOCASE,
    accepted INTEGER, -- Boolean
    follow_ap_id TEXT COLLATE NOCASE,
    actor_id INTEGER NOT NULL DEFAULT 0, -- Assuming FK to actors(id)
    UNIQUE(actor, leader_ap_id),
    FOREIGN KEY(actor_id) REFERENCES actors(id) -- Added based on common patterns, verify if intended
);

CREATE INDEX idx_leaders_actor_id ON leaders(actor_id);

CREATE TRIGGER leaders_auto_update_updated_at
AFTER UPDATE ON leaders
FOR EACH ROW
BEGIN
    UPDATE leaders
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
