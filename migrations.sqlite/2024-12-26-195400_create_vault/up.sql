CREATE TABLE vault (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    uuid TEXT NOT NULL,
    owner_as_id TEXT NOT NULL COLLATE NOCASE,
    activity_id INTEGER NOT NULL,
    data TEXT NOT NULL,
    FOREIGN KEY (activity_id) REFERENCES activities(id) ON DELETE CASCADE,
    FOREIGN KEY (owner_as_id) REFERENCES actors(as_id) ON DELETE CASCADE
);

CREATE TRIGGER vault_updated_at
    AFTER UPDATE ON vault FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE vault SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE UNIQUE INDEX uniq_vault_uuid ON vault (uuid);
CREATE INDEX vault_uuid ON vault (uuid);
CREATE INDEX vault_owner_as_id ON vault (owner_as_id);
CREATE INDEX vault_activity_id ON vault (activity_id);

