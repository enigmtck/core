CREATE TABLE remote_notes (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    kind TEXT CHECK(kind IN ('note','encrypted_note','vault_note','question')) NOT NULL,
    ap_id TEXT NOT NULL COLLATE NOCASE UNIQUE,
    published TEXT,
    url TEXT,
    ap_to TEXT,
    cc TEXT,
    tag TEXT,
    attributed_to TEXT NOT NULL,
    content TEXT NOT NULL,
    attachment TEXT,
    replies TEXT,
    in_reply_to TEXT,
    signature TEXT,
    summary TEXT,
    ap_sensitive BOOLEAN,
    atom_uri TEXT,
    in_reply_to_atom_uri TEXT,
    conversation TEXT,
    content_map TEXT
);

CREATE TRIGGER remote_notes_updated_at
    AFTER UPDATE ON remote_notes FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE remote_notes SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
