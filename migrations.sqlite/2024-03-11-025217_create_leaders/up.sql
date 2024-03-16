CREATE TABLE leaders (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    profile_id INTEGER NOT NULL,
    actor TEXT NOT NULL COLLATE NOCASE,
    leader_ap_id TEXT NOT NULL COLLATE NOCASE,
    uuid TEXT NOT NULL,
    accept_ap_id TEXT COLLATE NOCASE,
    accepted BOOLEAN,
    follow_ap_id TEXT COLLATE NOCASE,
    FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

CREATE UNIQUE INDEX uniq_actor_leader ON leaders (actor, leader_ap_id);

CREATE TRIGGER leaders_updated_at
    AFTER UPDATE ON leaders FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE leaders SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
