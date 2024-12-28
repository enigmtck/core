CREATE TABLE unprocessable (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    raw JSONB NOT NULL,
    error TEXT
);

CREATE TRIGGER unprocessable_updated_at
    AFTER UPDATE ON unprocessable FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE unprocessable SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

