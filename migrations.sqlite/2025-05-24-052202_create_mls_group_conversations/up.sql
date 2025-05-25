CREATE TABLE mls_group_conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    uuid TEXT NOT NULL,
    actor_id INTEGER NOT NULL,
    conversation TEXT NOT NULL,
    mls_group TEXT NOT NULL UNIQUE,
    FOREIGN KEY(actor_id) REFERENCES actors(id) ON DELETE CASCADE,
    UNIQUE(actor_id, conversation)
);

CREATE INDEX idx_mls_group_conversations_actor_id ON mls_group_conversations(actor_id);
CREATE INDEX idx_mls_group_conversations_created_at_asc ON mls_group_conversations(created_at);
CREATE INDEX idx_mls_group_conversations_uuid ON mls_group_conversations(uuid);


CREATE TRIGGER mls_group_conversations_auto_update_updated_at
AFTER UPDATE ON mls_group_conversations
FOR EACH ROW
BEGIN
    UPDATE mls_group_conversations
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
