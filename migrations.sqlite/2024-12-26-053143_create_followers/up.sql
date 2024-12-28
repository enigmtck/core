CREATE TABLE followers (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    ap_id TEXT NOT NULL COLLATE NOCASE,
    actor TEXT NOT NULL COLLATE NOCASE,
    followed_ap_id TEXT NOT NULL COLLATE NOCASE,
    uuid TEXT NOT NULL,
    actor_id INTEGER DEFAULT 0 NOT NULL
);

CREATE TRIGGER followers_updated_at
    AFTER UPDATE ON followers FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE followers SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE UNIQUE INDEX uniq_followers_ap_id ON followers (ap_id);
CREATE UNIQUE INDEX uniq_followers_uuid ON followers (uuid);
CREATE UNIQUE INDEX uniq_actor_followed_ap_id ON followers (actor, followed_ap_id);
CREATE INDEX idx_followers_actor_id ON followers (actor_id);
