CREATE TABLE remote_actors (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    context TEXT NOT NULL,
    kind TEXT NOT NULL,
    ap_id TEXT NOT NULL COLLATE NOCASE UNIQUE,
    name TEXT NOT NULL,
    preferred_username TEXT,
    summary TEXT,
    inbox TEXT NOT NULL COLLATE NOCASE,
    outbox TEXT NOT NULL COLLATE NOCASE,
    followers TEXT COLLATE NOCASE,
    following TEXT COLLATE NOCASE,
    liked TEXT COLLATE NOCASE,
    public_key TEXT,
    featured TEXT COLLATE NOCASE,
    featured_tags TEXT COLLATE NOCASE,
    url TEXT COLLATE NOCASE,
    manually_approves_followers BOOLEAN,
    published TEXT,
    tag TEXT,
    attachment TEXT,
    endpoints TEXT,
    icon TEXT,
    image TEXT,
    also_known_as TEXT,
    discoverable BOOLEAN,
    capabilities TEXT,
    checked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    webfinger TEXT COLLATE NOCASE
);

CREATE TRIGGER remote_actors_updated_at
    AFTER UPDATE ON remote_actors FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE remote_actors SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
