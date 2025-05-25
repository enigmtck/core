CREATE TABLE objects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')),
    ap_conversation TEXT,
    ap_sensitive INTEGER, -- Boolean
    ap_signature BLOB,
    ap_voters_count INTEGER,
    as_any_of BLOB,
    as_attachment BLOB,
    as_attributed_to BLOB,
    as_audience BLOB,
    as_bcc BLOB,
    as_bto BLOB,
    as_cc BLOB,
    as_closed BLOB, -- Could be timestamp or boolean based on usage, JSONB suggests flexible
    as_content TEXT,
    as_content_map BLOB,
    as_context BLOB,
    as_deleted TEXT, -- Timestamp
    as_describes BLOB,
    as_duration TEXT,
    as_end_time TEXT, -- Timestamp
    as_former_type TEXT,
    as_generator BLOB,
    as_icon BLOB,
    as_id TEXT NOT NULL UNIQUE COLLATE NOCASE,
    as_image BLOB,
    as_in_reply_to BLOB,
    as_location BLOB,
    as_media_type TEXT,
    as_name TEXT,
    as_name_map BLOB,
    as_one_of BLOB,
    as_preview BLOB,
    as_published TEXT, -- Timestamp
    as_replies BLOB,
    as_start_time TEXT, -- Timestamp
    as_summary TEXT,
    as_summary_map BLOB,
    as_tag BLOB,
    as_to BLOB,
    as_type TEXT NOT NULL CHECK (as_type IN ('article', 'audio', 'document', 'event', 'image', 'note', 'page', 'place', 'profile', 'question', 'relationship', 'tombstone', 'video', 'encrypted_note')),
    as_updated TEXT, -- Timestamp
    as_url BLOB,
    ek_hashtags BLOB NOT NULL DEFAULT '[]',
    ek_instrument BLOB,
    ek_metadata BLOB,
    ek_profile_id INTEGER, -- Assuming FK to actors(id)
    ek_uuid TEXT,
    FOREIGN KEY(ek_profile_id) REFERENCES actors(id) -- Added based on common patterns
);

CREATE INDEX idx_attributed_to ON objects(as_attributed_to);
CREATE INDEX idx_objects_created_at_desc ON objects(created_at DESC); -- Renamed from idx_created_at_desc
CREATE INDEX idx_in_reply_to ON objects(as_in_reply_to);
CREATE INDEX idx_object_ek_hashtags ON objects(ek_hashtags);
CREATE INDEX idx_objects_ap_conversation ON objects(ap_conversation);
CREATE INDEX idx_objects_as_bcc ON objects(as_bcc);
CREATE INDEX idx_objects_as_bto ON objects(as_bto);
CREATE INDEX idx_objects_as_cc ON objects(as_cc);
CREATE INDEX idx_objects_as_type_as_published ON objects(as_type, as_published DESC);
CREATE INDEX idx_objects_ek_profile_id ON objects(ek_profile_id);
CREATE INDEX idx_objects_ek_uuid ON objects(ek_uuid);
CREATE INDEX idx_objects_type ON objects(as_type) WHERE (as_type <> 'tombstone');
CREATE INDEX idx_to ON objects(as_to);
CREATE INDEX idx_objects_type_created_at_desc ON objects(as_type, created_at DESC); -- Renamed from idx_type_created_at_desc


CREATE TRIGGER objects_auto_update_updated_at
AFTER UPDATE ON objects
FOR EACH ROW
BEGIN
    UPDATE objects
    SET updated_at = STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')
    WHERE rowid = NEW.rowid;
END;
