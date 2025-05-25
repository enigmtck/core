CREATE TABLE notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    uuid TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('mention', 'announce', 'unannounce', 'like', 'unlike', 'follow', 'unfollow', 'accept', 'block')),
    profile_id INTEGER NOT NULL, -- Assuming FK to actors(id)
    activity_id INTEGER NOT NULL, -- Assuming FK to activities(id)
    FOREIGN KEY(profile_id) REFERENCES actors(id), -- Added based on common patterns
    FOREIGN KEY(activity_id) REFERENCES activities(id) -- Added based on common patterns
);

CREATE TRIGGER notifications_auto_update_updated_at
AFTER UPDATE ON notifications
FOR EACH ROW
BEGIN
    UPDATE notifications
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
