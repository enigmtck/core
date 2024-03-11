CREATE TABLE follows (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    uuid TEXT NOT NULL,
    profile_id INTEGER,
    ap_object TEXT NOT NULL,
    actor TEXT NOT NULL
);

CREATE TRIGGER follows_updated_at
    AFTER UPDATE ON follows FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE follows SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
