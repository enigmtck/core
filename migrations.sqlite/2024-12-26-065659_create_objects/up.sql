CREATE TABLE objects (
    id INTEGER PRIMARY KEY NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    ap_conversation TEXT,
    ap_sensitive BOOLEAN,
    ap_signature JSONB,
    ap_voters_count INTEGER,
    as_any_of JSONB,
    as_attachment JSONB,
    as_attributed_to JSONB,
    as_audience JSONB,
    as_bcc JSONB,
    as_bto JSONB,
    as_cc JSONB,
    as_closed JSONB,
    as_content TEXT,
    as_content_map JSONB,
    as_conTEXT JSONB,
    as_deleted TIMESTAMP,
    as_describes JSONB,
    as_duration TEXT,
    as_end_time TIMESTAMP,
    as_former_type TEXT,
    as_generator JSONB,
    as_icon JSONB,
    as_id TEXT NOT NULL COLLATE NOCASE,
    as_image JSONB,
    as_in_reply_to JSONB,
    as_location JSONB,
    as_media_type TEXT,
    as_name TEXT,
    as_name_map JSONB,
    as_one_of JSONB,
    as_preview JSONB,
    as_published TIMESTAMP,
    as_replies JSONB,
    as_start_time TIMESTAMP,
    as_summary TEXT,
    as_summary_map JSONB,
    as_tag JSONB,
    as_to JSONB,
    as_type TEXT CHECK(as_type IN ('article','audio','document','event','image','note','page','place','profile','question','relationship','tombstone','video','encrypted_note')) NOT NULL,
    as_updated TIMESTAMP,
    as_url JSONB,
    ek_hashtags JSONB DEFAULT '[]' NOT NULL,
    ek_instrument JSONB,
    ek_metadata JSONB,
    ek_profile_id INTEGER,
    ek_uuid TEXT
);

CREATE TRIGGER objects_updated_at
    AFTER UPDATE ON objects FOR EACH ROW
    WHEN OLD.updated_at = NEW.updated_at OR OLD.updated_at IS NULL
BEGIN
    UPDATE objects SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id;
END;

CREATE UNIQUE INDEX uniq_objects_as_id ON objects (as_id);
CREATE INDEX idx_attributed_to ON objects (json_extract(as_attributed_to, '$[*]'));
CREATE INDEX idx_created_at_desc ON objects (created_at DESC);
CREATE INDEX idx_in_reply_to ON objects (json_extract(as_in_reply_to, '$[*]'));
CREATE INDEX idx_object_ek_hashtags ON objects (json_extract(ek_hashtags, '$[*]'));
CREATE INDEX idx_objects_ap_conversation ON objects (ap_conversation);
CREATE INDEX idx_objects_as_bcc ON objects (json_extract(as_bcc, '$[*]'));
CREATE INDEX idx_objects_as_bto ON objects (json_extract(as_bto, '$[*]'));
CREATE INDEX idx_objects_as_cc ON objects (json_extract(as_cc, '$[*]'));
CREATE INDEX idx_objects_as_type_as_published ON objects (as_type, as_published DESC);
CREATE INDEX idx_objects_ek_profile_id ON objects (ek_profile_id);
CREATE INDEX idx_objects_ek_uuid ON objects (ek_uuid);
CREATE INDEX idx_to ON objects (json_extract(as_to, '$[*]'));
CREATE INDEX idx_type_created_at_desc ON objects (as_type, created_at DESC);

