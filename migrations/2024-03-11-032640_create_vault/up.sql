CREATE TABLE vault (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    uuid TEXT NOT NULL UNIQUE,
    profile_id INTEGER NOT NULL,
    encrypted_data TEXT NOT NULL,
    remote_actor TEXT NOT NULL,
    outbound BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

CREATE TRIGGER vault_updated_at
    AFTER UPDATE ON vault FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE vault SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
