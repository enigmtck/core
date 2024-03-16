CREATE TABLE notes (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    uuid TEXT NOT NULL UNIQUE,
    profile_id INTEGER NOT NULL,
    kind TEXT CHECK(kind IN ('note','encrypted_note','vault_note','question')) NOT NULL,
    ap_to TEXT NOT NULL,
    cc TEXT,
    tag TEXT,
    attributed_to TEXT NOT NULL,
    in_reply_to TEXT,
    content TEXT NOT NULL,
    conversation TEXT,
    attachment TEXT,
    instrument TEXT,
    ap_id TEXT,
    FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

CREATE TRIGGER notes_updated_at
    AFTER UPDATE ON notes FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE notes SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
