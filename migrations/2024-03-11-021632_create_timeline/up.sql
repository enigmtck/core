CREATE TABLE timeline (
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

CREATE TABLE timeline_cc (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    timeline_id INTEGER NOT NULL,
    ap_id TEXT NOT NULL COLLATE NOCASE,
    FOREIGN KEY(timeline_id) REFERENCES timeline(id) ON DELETE CASCADE
);

CREATE INDEX idx_timeline_cc_ap_id ON timeline_cc (ap_id);
CREATE INDEX idx_timeline_cc_timeline_id ON timeline_cc (timeline_id);
CREATE UNIQUE INDEX uniq_timeline_cc_id_ap_id ON timeline_cc (timeline_id, ap_id);

CREATE TRIGGER timeline_cc_updated_at
    AFTER UPDATE ON timeline_cc FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE timeline_cc SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE TABLE timeline_to (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    timeline_id INTEGER NOT NULL,
    ap_id TEXT NOT NULL COLLATE NOCASE,
    FOREIGN KEY(timeline_id) REFERENCES timeline(id) ON DELETE CASCADE
);

CREATE INDEX idx_timeline_to_ap_id ON timeline_to (ap_id);
CREATE INDEX idx_timeline_to_timeline_id ON timeline_to (timeline_id);
CREATE UNIQUE INDEX uniq_timeline_to_id_ap_id ON timeline_to (timeline_id, ap_id);

CREATE TRIGGER timeline_to_updated_at
    AFTER UPDATE ON timeline_to FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE timeline_to SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
