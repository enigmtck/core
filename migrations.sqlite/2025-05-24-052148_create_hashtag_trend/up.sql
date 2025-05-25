CREATE TABLE hashtag_trend (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    period INTEGER NOT NULL,
    hashtag TEXT NOT NULL COLLATE NOCASE,
    update_count INTEGER NOT NULL DEFAULT 0
);

CREATE TRIGGER hashtag_trend_auto_update_updated_at
AFTER UPDATE ON hashtag_trend
FOR EACH ROW
BEGIN
    UPDATE hashtag_trend
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;

CREATE TRIGGER hashtag_trend_auto_increment_update_count
AFTER UPDATE ON hashtag_trend
FOR EACH ROW
BEGIN
    UPDATE hashtag_trend
    SET update_count = OLD.update_count + 1
    WHERE rowid = NEW.rowid;
END;
