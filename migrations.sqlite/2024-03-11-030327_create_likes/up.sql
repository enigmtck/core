CREATE TABLE likes (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    uuid TEXT NOT NULL,
    profile_id INTEGER,
    ap_to TEXT NOT NULL,
    actor TEXT NOT NULL,
    object_ap_id TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_announces_actor_object_ap_id ON likes (actor, object_ap_id);
CREATE INDEX idx_likes_object_ap_id ON likes (object_ap_id);
CREATE INDEX idx_likes_profile_id ON likes (profile_id);

CREATE TRIGGER likes_updated_at
    AFTER UPDATE ON likes FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE likes SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
