CREATE TABLE actors (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
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
    ek_checked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    ek_hashtags JSONB DEFAULT '[]' NOT NULL,
    as_type TEXT CHECK(as_type IN ('person','service','group','organization','application')) NOT NULL,
    as_context JSONB,
    as_id TEXT NOT NULL COLLATE NOCASE,
    as_name TEXT COLLATE NOCASE,
    as_preferred_username TEXT COLLATE NOCASE,
    as_summary TEXT,
    as_inbox TEXT NOT NULL COLLATE NOCASE,
    as_outbox TEXT NOT NULL COLLATE NOCASE,
    as_followers TEXT COLLATE NOCASE,
    as_following TEXT COLLATE NOCASE,
    as_liked TEXT COLLATE NOCASE,
    as_public_key JSONB NOT NULL,
    as_featured TEXT COLLATE NOCASE,
    as_featured_tags TEXT COLLATE NOCASE,
    as_url JSONB,
    as_published TIMESTAMP,
    as_tag JSONB DEFAULT '[]' NOT NULL,
    as_attachment JSONB DEFAULT '[]' NOT NULL,
    as_endpoints JSONB DEFAULT '{}' NOT NULL,
    as_icon JSONB DEFAULT '{}' NOT NULL,
    as_image JSONB DEFAULT '{}' NOT NULL,
    as_also_known_as JSONB DEFAULT '[]' NOT NULL,
    as_discoverable BOOLEAN DEFAULT 1 NOT NULL,
    ap_capabilities JSONB DEFAULT '{}' NOT NULL,
    ap_manually_approves_followers BOOLEAN DEFAULT 0 NOT NULL,
    ek_keys TEXT,
    ek_last_decrypted_activity TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TRIGGER actors_updated_at
    AFTER UPDATE ON actors FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE actors SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE UNIQUE INDEX uniq_actors_as_id ON actors (as_id);
CREATE INDEX idx_actors_as_followers ON actors (as_followers);
CREATE INDEX idx_actors_as_following ON actors (as_following);
CREATE INDEX idx_actors_as_type ON actors (as_type);
CREATE INDEX idx_actors_checked_at ON actors (ek_checked_at DESC);
CREATE INDEX idx_actors_created_at ON actors (created_at DESC);
CREATE INDEX idx_actors_ek_hashtags ON actors (json_extract(ek_hashtags, '$[*]'));
CREATE INDEX idx_actors_ek_uuid ON actors (ek_uuid);
CREATE INDEX idx_actors_ek_webfinger ON actors (ek_webfinger);
CREATE INDEX idx_actors_public_key ON actors (json_extract(as_public_key, '$.keyId'));
CREATE INDEX idx_actors_updated_at ON actors (updated_at DESC);

