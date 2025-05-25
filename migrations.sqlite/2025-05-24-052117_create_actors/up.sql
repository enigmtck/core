CREATE TABLE actors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    ek_uuid TEXT,
    ek_username TEXT COLLATE NOCASE,
    ek_summary_markdown TEXT,
    ek_avatar_filename TEXT,
    ek_banner_filename TEXT,
    ek_private_key TEXT,
    ek_password TEXT,
    ek_client_public_key TEXT,
    ek_client_private_key TEXT,
    ek_salt TEXT,
    ek_olm_pickled_account TEXT,
    ek_olm_pickled_account_hash TEXT,
    ek_olm_identity_key TEXT,
    ek_webfinger TEXT COLLATE NOCASE,
    ek_checked_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    ek_hashtags BLOB NOT NULL DEFAULT '[]',
    as_type TEXT NOT NULL CHECK (as_type IN ('person', 'service', 'group', 'organization', 'application', 'tombstone')),
    as_context BLOB,
    as_id TEXT NOT NULL UNIQUE COLLATE NOCASE,
    as_name TEXT COLLATE NOCASE,
    as_preferred_username TEXT COLLATE NOCASE,
    as_summary TEXT,
    as_inbox TEXT NOT NULL COLLATE NOCASE,
    as_outbox TEXT NOT NULL COLLATE NOCASE,
    as_followers TEXT COLLATE NOCASE,
    as_following TEXT COLLATE NOCASE,
    as_liked TEXT COLLATE NOCASE,
    as_public_key BLOB NOT NULL,
    as_featured TEXT COLLATE NOCASE,
    as_featured_tags TEXT COLLATE NOCASE,
    as_url BLOB,
    as_published TEXT, -- Timestamp
    as_tag BLOB NOT NULL DEFAULT '[]',
    as_attachment BLOB NOT NULL DEFAULT '[]',
    as_endpoints BLOB NOT NULL DEFAULT '{}',
    as_icon BLOB NOT NULL DEFAULT '{}',
    as_image BLOB NOT NULL DEFAULT '{}',
    as_also_known_as BLOB NOT NULL DEFAULT '[]',
    as_discoverable INTEGER NOT NULL DEFAULT 1,
    ap_capabilities BLOB NOT NULL DEFAULT '{}',
    ap_manually_approves_followers INTEGER NOT NULL DEFAULT 0,
    ek_keys TEXT,
    ek_last_decrypted_activity TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    ek_mls_credentials TEXT,
    ek_mls_storage TEXT,
    ek_mls_storage_hash TEXT
);

CREATE INDEX idx_actors_as_followers ON actors(as_followers);
CREATE INDEX idx_actors_as_following ON actors(as_following);
CREATE INDEX idx_actors_as_type ON actors(as_type);
CREATE INDEX idx_actors_checked_at ON actors(ek_checked_at DESC);
CREATE INDEX idx_actors_created_at ON actors(created_at DESC);
CREATE INDEX idx_actors_ek_hashtags ON actors(ek_hashtags); -- Indexing BLOB
CREATE INDEX idx_actors_ek_uuid ON actors(ek_uuid);
CREATE INDEX idx_actors_ek_webfinger ON actors(ek_webfinger);
CREATE INDEX idx_actors_public_key ON actors(as_public_key); -- Indexing BLOB
CREATE INDEX idx_actors_updated_at ON actors(updated_at DESC);

CREATE TRIGGER actors_auto_update_updated_at
AFTER UPDATE ON actors
FOR EACH ROW
BEGIN
    UPDATE actors
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
