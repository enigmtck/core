CREATE TABLE followers (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    profile_id INTEGER NOT NULL,
    ap_id TEXT NOT NULL COLLATE NOCASE UNIQUE,
    actor TEXT NOT NULL COLLATE NOCASE,
    followed_ap_id TEXT NOT NULL COLLATE NOCASE,
    uuid TEXT NOT NULL UNIQUE,
    FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

CREATE INDEX idx_followers_actor_followed ON followers (actor, followed_ap_id);

CREATE TRIGGER followers_updated_at
    AFTER UPDATE ON followers FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE followers SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
