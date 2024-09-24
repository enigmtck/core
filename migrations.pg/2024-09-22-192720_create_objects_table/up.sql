CREATE TYPE object_type AS ENUM ('article', 'audio', 'document', 'event', 'image', 'note', 'page', 'place', 'profile', 'question', 'relationship', 'tombstone', 'video');

CREATE TABLE objects (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  ap_conversation TEXT,
  ap_sensitive BOOLEAN,
  ap_signature JSONB,
  ap_voters_count INT,
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
  as_context JSONB,
  as_deleted TIMESTAMPTZ,
  as_describes JSONB,
  as_duration TEXT,
  as_end_time TIMESTAMPTZ,
  as_former_type TEXT,
  as_generator JSONB,
  as_icon JSONB,
  as_id TEXT NOT NULL COLLATE "case_insensitive",
  as_image JSONB,
  as_in_reply_to JSONB,
  as_location JSONB,
  as_media_type TEXT,
  as_name TEXT,
  as_name_map JSONB,
  as_one_of JSONB,
  as_preview JSONB,
  as_published TIMESTAMPTZ,
  as_replies JSONB,
  as_start_time TIMESTAMPTZ,
  as_summary TEXT,
  as_summary_map JSONB,
  as_tag JSONB,
  as_to JSONB,
  as_type object_type NOT NULL,
  as_updated TIMESTAMPTZ,
  as_url JSONB,
  ek_hashtags JSONB NOT NULL DEFAULT '[]',
  ek_instrument JSONB,
  ek_metadata JSONB,
  ek_profile_id INT,
  ek_uuid TEXT
);

SELECT diesel_manage_updated_at('objects');

CREATE INDEX idx_created_at_desc ON objects USING btree (created_at DESC);
CREATE INDEX idx_type_created_at_desc ON objects USING btree (as_type, created_at DESC);
CREATE INDEX idx_attributed_to ON objects USING gin (as_attributed_to);
CREATE INDEX idx_in_reply_to ON objects USING gin (as_in_reply_to);
CREATE INDEX idx_to ON objects USING gin (as_to);

INSERT INTO objects (created_at, updated_at, ek_uuid, ek_profile_id, as_type, as_to, as_cc, as_tag, as_attributed_to, as_in_reply_to, as_content, ap_conversation, as_attachment, ek_instrument, as_id) SELECT created_at, updated_at, uuid, profile_id, 'note', ap_to, cc, tag, to_jsonb(attributed_to), to_jsonb(in_reply_to), content, conversation, attachment, instrument, ap_id FROM notes;

INSERT INTO objects (created_at, updated_at, as_type, as_id, as_published, as_url, as_to, as_cc, as_tag, as_attributed_to, as_content, as_attachment, as_replies, as_in_reply_to, ap_signature, as_summary, ap_sensitive, ap_conversation, as_content_map, ek_metadata, ek_hashtags)
SELECT 
    rn.created_at,
    rn.updated_at,
    'note',
    rn.ap_id,
    rn.published::timestamp,
    to_jsonb(rn.url),
    rn.ap_to,
    rn.cc,
    rn.tag,
    to_jsonb(rn.attributed_to),
    rn.content,
    rn.attachment,
    rn.replies,
    to_jsonb(rn.in_reply_to),
    rn.signature,
    rn.summary,
    rn.ap_sensitive,
    rn.conversation,
    rn.content_map,
    rn.metadata,
    COALESCE(rnh.hashtags, '[]') AS ek_hashtags
FROM remote_notes rn
LEFT JOIN (
    SELECT remote_note_id, jsonb_agg(hashtag) AS hashtags
    FROM remote_note_hashtags
    GROUP BY remote_note_id
) rnh ON rn.id = rnh.remote_note_id;

INSERT INTO objects (created_at, updated_at, as_type, as_id, as_to, as_cc, as_end_time, as_published, as_one_of, as_any_of, as_content, as_content_map, as_summary, ap_voters_count, as_url, ap_conversation, as_tag, as_attachment, ap_sensitive, as_in_reply_to, as_attributed_to) SELECT created_at, updated_at, 'question', ap_id, ap_to, cc, end_time, published, one_of, any_of, content, content_map, summary, voters_count, to_jsonb(url), conversation, tag, attachment, ap_sensitive, to_jsonb(in_reply_to), to_jsonb(attributed_to) FROM remote_questions;
