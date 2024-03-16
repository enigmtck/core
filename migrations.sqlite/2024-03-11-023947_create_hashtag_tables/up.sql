CREATE TABLE remote_note_hashtags (
  id INTEGER PRIMARY KEY NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  hashtag TEXT NOT NULL COLLATE NOCASE,
  remote_note_id INTEGER NOT NULL,
  FOREIGN KEY(remote_note_id) REFERENCES remote_notes(id) ON DELETE CASCADE
);

CREATE TRIGGER remote_note_hashtags_updated_at
    AFTER UPDATE ON remote_note_hashtags FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE remote_note_hashtags SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE TABLE remote_actor_hashtags (
  id INTEGER PRIMARY KEY NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  hashtag TEXT NOT NULL COLLATE NOCASE,
  remote_actor_id INTEGER NOT NULL,
  FOREIGN KEY(remote_actor_id) REFERENCES remote_actors(id) ON DELETE CASCADE
);

CREATE TRIGGER remote_actor_hashtags_updated_at
    AFTER UPDATE ON remote_actor_hashtags FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE remote_actor_hashtags SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE TABLE note_hashtags (
  id INTEGER PRIMARY KEY NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  hashtag TEXT NOT NULL COLLATE NOCASE,
  note_id INTEGER NOT NULL,
  FOREIGN KEY(note_id) REFERENCES remote_notes(id) ON DELETE CASCADE
);

CREATE TRIGGER note_hashtags_updated_at
    AFTER UPDATE ON note_hashtags FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE note_hashtags SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE TABLE profile_hashtags (
  id INTEGER PRIMARY KEY NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  hashtag TEXT NOT NULL COLLATE NOCASE,
  profile_id INTEGER NOT NULL,
  FOREIGN KEY(profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE TRIGGER profile_hashtags_updated_at
    AFTER UPDATE ON profile_hashtags FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE profile_hashtags SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE TABLE timeline_hashtags (
  id INTEGER PRIMARY KEY NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  hashtag TEXT NOT NULL COLLATE NOCASE,
  timeline_id INTEGER NOT NULL,
  FOREIGN KEY(timeline_id) REFERENCES timeline(id) ON DELETE CASCADE
);

CREATE TRIGGER timeline_hashtags_updated_at
    AFTER UPDATE ON timeline_hashtags FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE timeline_hashtags SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE TABLE hashtag_trend (
  id INTEGER PRIMARY KEY NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  period INTEGER NOT NULL,
  hashtag TEXT NOT NULL COLLATE NOCASE,
  update_count INTEGER NOT NULL DEFAULT 0
);

CREATE TRIGGER hashtag_trend_updated_at
    AFTER UPDATE ON hashtag_trend FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE hashtag_trend SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
