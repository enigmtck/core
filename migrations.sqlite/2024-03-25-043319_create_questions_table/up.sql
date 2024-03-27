CREATE TABLE remote_questions (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    kind TEXT CHECK(kind IN ('question')) NOT NULL,
    ap_id TEXT NOT NULL COLLATE NOCASE UNIQUE,
    ap_to TEXT,
    cc TEXT,
    end_time TIMESTAMP,
    published TIMESTAMP,
    one_of TEXT,
    any_of TEXT,
    content TEXT,
    content_map TEXT,
    summary TEXT,
    voters_count INTEGER,
    url TEXT,
    conversation TEXT,
    tag TEXT,
    attachment TEXT,
    ap_sensitive BOOLEAN,
    in_reply_to TEXT
);

CREATE TRIGGER questions_updated_at
    AFTER UPDATE ON remote_questions FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE remote_questions SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
