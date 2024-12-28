CREATE TABLE notifications (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    uuid TEXT NOT NULL,
    kind TEXT CHECK(kind IN ('mention','announce','unannounce','like','unlike','follow','unfollow','accept','block')) NOT NULL,
    profile_id INTEGER NOT NULL,
    activity_id INTEGER NOT NULL
);

CREATE TRIGGER notifications_updated_at
    AFTER UPDATE ON notifications FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE notifications SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
