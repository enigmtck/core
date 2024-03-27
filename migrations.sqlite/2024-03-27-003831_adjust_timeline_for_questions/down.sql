PRAGMA foreign_keys=off;

CREATE TABLE IF NOT EXISTS new_timeline( 
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    tag TEXT,
    attributed_to TEXT NOT NULL COLLATE NOCASE,
    ap_id TEXT NOT NULL COLLATE NOCASE UNIQUE,
    kind TEXT CHECK(kind IN ('note','encrypted_note','vault_note','question')) NOT NULL,
    url TEXT,
    published TEXT,
    replies TEXT,
    in_reply_to TEXT,
    content TEXT,
    ap_public BOOLEAN DEFAULT 0 NOT NULL,
    summary TEXT,
    ap_sensitive BOOLEAN,
    atom_uri TEXT,
    in_reply_to_atom_uri TEXT,
    conversation TEXT,
    content_map TEXT,
    attachment TEXT,
    ap_object TEXT,
    metadata TEXT
);

INSERT INTO new_timeline(id, created_at, updated_at, tag, attributed_to, ap_id, kind, url, published, replies, in_reply_to, content, ap_public, summary, ap_sensitive, atom_uri, in_reply_to_atom_uri, conversation, content_map, attachment, ap_object, metadata)
SELECT id, created_at, updated_at, tag, attributed_to, ap_id, kind, url, published, replies, in_reply_to, content, ap_public, summary, ap_sensitive, atom_uri, in_reply_to_atom_uri, conversation, content_map, attachment, ap_object, metadata
FROM timeline;

DROP TABLE timeline;
ALTER TABLE new_timeline RENAME TO timeline;

CREATE INDEX idx_timeline_ap_id ON timeline (ap_id);
CREATE INDEX idx_timeline_conversation ON timeline (conversation);
CREATE INDEX idx_timeline_in_reply_to ON timeline (in_reply_to);
CREATE INDEX idx_timeline_ap_public_created_at ON timeline (ap_public, created_at DESC);
CREATE INDEX idx_timeline_created_at ON timeline (created_at);


CREATE TRIGGER timeline_updated_at
    AFTER UPDATE ON timeline FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE timeline SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

PRAGMA foreign_keys=on;
