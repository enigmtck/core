CREATE TABLE instances (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    domain_name TEXT NOT NULL COLLATE NOCASE,
    "json" JSONB,
    blocked BOOLEAN DEFAULT 0 NOT NULL,
    last_message_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    shared_inbox TEXT
);

CREATE TRIGGER instances_updated_at
    AFTER UPDATE ON instances FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE instances SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE UNIQUE INDEX uniq_instances_domain_name ON instances (domain_name);

