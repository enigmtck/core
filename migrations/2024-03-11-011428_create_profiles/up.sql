CREATE TABLE profiles (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    uuid TEXT NOT NULL UNIQUE,
    username TEXT NOT NULL COLLATE NOCASE UNIQUE,
    display_name TEXT NOT NULL,
    summary TEXT,
    public_key TEXT NOT NULL,
    private_key TEXT NOT NULL,
    password TEXT,
    client_public_key TEXT,
    avatar_filename TEXT NOT NULL DEFAULT 'default.png',
    banner_filename TEXT,
    salt TEXT,
    client_private_key TEXT,
    olm_pickled_account TEXT,
    olm_pickled_account_hash TEXT,
    olm_identity_key TEXT,
    summary_markdown TEXT
);

CREATE TRIGGER profiles_updated_at
    AFTER UPDATE ON profiles FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE profiles SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;
