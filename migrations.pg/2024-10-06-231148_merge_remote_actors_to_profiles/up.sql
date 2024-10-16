CREATE TYPE actor_type AS ENUM ('person', 'service', 'group', 'organization', 'application');

CREATE TABLE actors (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  ek_uuid VARCHAR,
  ek_username VARCHAR COLLATE "case_insensitive",
  ek_summary_markdown VARCHAR,
  ek_avatar_filename VARCHAR,
  ek_banner_filename VARCHAR,
  ek_private_key VARCHAR,
  ek_password VARCHAR,
  ek_client_public_key VARCHAR,
  ek_client_private_key VARCHAR,
  ek_salt VARCHAR,
  ek_olm_pickled_account VARCHAR,
  ek_olm_pickled_account_hash VARCHAR,
  ek_olm_identity_key VARCHAR,
  ek_webfinger VARCHAR COLLATE "case_insensitive",
  ek_checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  ek_hashtags JSONB NOT NULL DEFAULT '[]'::jsonb,
  as_type actor_type NOT NULL,
  as_context JSONB,
  as_id VARCHAR NOT NULL COLLATE "case_insensitive" UNIQUE,
  as_name VARCHAR COLLATE "case_insensitive",
  as_preferred_username VARCHAR COLLATE "case_insensitive",
  as_summary VARCHAR,
  as_inbox VARCHAR NOT NULL COLLATE "case_insensitive",
  as_outbox VARCHAR NOT NULL COLLATE "case_insensitive",
  as_followers VARCHAR COLLATE "case_insensitive",
  as_following VARCHAR COLLATE "case_insensitive",
  as_liked VARCHAR COLLATE "case_insensitive",
  as_public_key JSONB NOT NULL,
  as_featured VARCHAR COLLATE "case_insensitive",
  as_featured_tags VARCHAR COLLATE "case_insensitive",
  as_url VARCHAR COLLATE "case_insensitive",
  as_published TIMESTAMPTZ,
  as_tag JSONB NOT NULL DEFAULT '[]'::jsonb,
  as_attachment JSONB NOT NULL DEFAULT '[]'::jsonb,
  as_endpoints JSONB NOT NULL DEFAULT '{}'::jsonb,
  as_icon JSONB NOT NULL DEFAULT '{}'::jsonb,
  as_image JSONB NOT NULL DEFAULT '{}'::jsonb,
  as_also_known_as JSONB NOT NULL DEFAULT '[]'::jsonb,
  as_discoverable BOOLEAN NOT NULL DEFAULT 'true',
  ap_capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
  ap_manually_approves_followers BOOLEAN NOT NULL DEFAULT 'false'
);

CREATE INDEX idx_actors_ek_uuid ON actors (ek_uuid);
CREATE INDEX idx_actors_updated_at ON actors (updated_at DESC);
CREATE INDEX idx_actors_created_at ON actors (created_at DESC);
CREATE INDEX idx_actors_checked_at ON actors (ek_checked_at DESC);
CREATE INDEX idx_actors_ek_hashtags ON actors USING gin (ek_hashtags);
CREATE INDEX idx_actors_ek_webfinger ON actors (ek_webfinger);
CREATE INDEX idx_actors_as_followers ON actors (as_followers);
CREATE INDEX idx_actors_as_following ON actors (as_following);
CREATE INDEX idx_actors_as_type ON actors (as_type);

INSERT INTO actors (
  created_at,
  as_type,
  ek_uuid,
  ek_username,
  as_name,
  as_summary,
  as_public_key,
  ek_private_key,
  ek_password,
  ek_client_public_key,
  ek_avatar_filename,
  ek_banner_filename,
  ek_salt,
  ek_client_private_key,
  ek_olm_pickled_account,
  ek_olm_pickled_account_hash,
  ek_olm_identity_key,
  ek_summary_markdown,
  as_preferred_username,
  as_inbox,
  as_outbox,
  as_followers,
  as_following,
  as_liked,
  as_published,
  as_url,
  as_endpoints,
  as_discoverable,
  ap_manually_approves_followers,
  ap_capabilities,
  as_also_known_as,
  as_tag,
  as_id,
  as_icon,
  as_image,
  ek_webfinger
)
SELECT
  created_at,
  'person',
  uuid,
  username,
  display_name,
  summary,
  ('{"id": "https://enigmatick.jdt.dev/user/' || username || '#main-key", "owner": "https://enigmatick.jdt.dev/user/' || username || '", "publicKeyPem": "' || regexp_replace(public_key, E'\n', '\\n', 'g') || '"}')::jsonb,
  private_key,
  password,
  client_public_key,
  avatar_filename,
  banner_filename,
  salt,
  client_private_key,
  olm_pickled_account,
  olm_pickled_account_hash,
  olm_identity_key,
  summary_markdown,
  username,
  'https://enigmatick.jdt.dev/user/' || username || '/inbox',
  'https://enigmatick.jdt.dev/user/' || username || '/outbox',
  'https://enigmatick.jdt.dev/user/' || username || '/followers',
  'https://enigmatick.jdt.dev/user/' || username || '/following',
  'https://enigmatick.jdt.dev/user/' || username || '/liked',
  created_at,
  'https://enigmatick.jdt.dev/@' || username,
  '{"sharedInbox": "https://enigmatick.jdt.dev/inbox"}'::jsonb,
  true,
  false,
  '{"acceptsChatMessages": false, "enigmatickEncryption": true}'::jsonb,
  '[]'::jsonb,
  '[]'::jsonb,
  'https://enigmatick.jdt.dev/user/' || username,
  coalesce(('{"url": "https://enigmatick.jdt.dev/' || avatar_filename || '", "type": "Image", "mediaType": "image/png"}')::jsonb, '{}'::jsonb),
  coalesce(('{"url": "https://enigmatick.jdt.dev/media/banners/' || banner_filename || '", "type": "Image", "mediaType": "image/png"}')::jsonb, '{}'::jsonb),
  '@' || username || '@enigmatick.jdt.dev'
  FROM profiles;

INSERT INTO actors (
  created_at,
  as_context,
  as_type,
  as_id,
  as_name,
  as_preferred_username,
  as_summary,
  as_inbox,
  as_outbox,
  as_followers,
  as_following,
  as_liked,
  as_public_key,
  as_featured,
  as_featured_tags,
  as_url,
  ap_manually_approves_followers,
  as_published,
  as_tag,
  as_attachment,
  as_endpoints,
  as_icon,
  as_image,
  as_also_known_as,
  as_discoverable,
  ap_capabilities,
  ek_webfinger
)
SELECT
  created_at,
  context,
  lower(kind)::actor_type,
  ap_id,
  name,
  preferred_username,
  summary,
  inbox,
  outbox,
  followers,
  following,
  liked,
  public_key,
  featured,
  featured_tags,
  url,
  coalesce(manually_approves_followers, false),
  published::timestamptz,
  coalesce(tag, '[]'::jsonb),
  coalesce(attachment, '[]'::jsonb),
  coalesce(endpoints, '{}'::jsonb),
  coalesce(icon, '{}'::jsonb),
  coalesce(image, '{}'::jsonb),
  coalesce(also_known_as, '[]'::jsonb),
  coalesce(discoverable, true),
  coalesce(capabilities, '{}'::jsonb),
  webfinger
  FROM remote_actors
         ON CONFLICT (as_id) DO NOTHING;

ALTER TABLE followers ADD COLUMN actor_id INT NOT NULL DEFAULT 0;
UPDATE followers f SET actor_id = COALESCE((SELECT id FROM actors a where a.as_id = f.followed_ap_id LIMIT 1), 0);
CREATE INDEX idx_followers_actor_id ON followers (actor_id);

ALTER TABLE leaders ADD COLUMN actor_id INT NOT NULL DEFAULT 0;
UPDATE leaders l SET actor_id = COALESCE((SELECT id from actors a where a.as_id = l.actor LIMIT 1), 0);
CREATE INDEX idx_leaders_actor_id ON leaders (actor_id);

ALTER TABLE activities ADD COLUMN actor_id INT REFERENCES actors (id);
UPDATE activities a
   SET actor_id = (
     SELECT ac.id
       FROM activities a2
            LEFT JOIN profiles p
                ON (a2.profile_id = p.id)
            INNER JOIN actors ac
                ON (ac.as_id = 'https://enigmatick.jdt.dev/user/' || p.username)
      WHERE a2.id = a.id)
 WHERE a.profile_id IS NOT NULL;
CREATE INDEX idx_activities_actor_id ON activities (actor_id);

ALTER TABLE activities ADD COLUMN target_actor_id INT REFERENCES actors (id);
UPDATE activities a
   SET target_actor_id = (
     SELECT ac.id
       FROM activities a2
            LEFT JOIN remote_actors r
                ON (a2.target_remote_actor_id = r.id)
            INNER JOIN actors ac
                ON (ac.as_id = r.ap_id)
      WHERE a2.id = a.id)
 WHERE a.target_remote_actor_id IS NOT NULL;
CREATE INDEX idx_activities_target_actor_id ON activities (target_actor_id);

UPDATE activities a
   SET target_object_id = (
     SELECT o.id
       FROM activities a2
            INNER JOIN objects o
                ON (a.target_ap_id = o.as_id)
      WHERE a2.id = a.id
   )
 WHERE a.target_object_id IS NULL;

DELETE FROM activities
 WHERE target_note_id IS NULL
   AND target_remote_note_id IS NULL
   AND target_profile_id IS NULL
   AND target_activity_id IS NULL
   AND target_remote_actor_id IS NULL
   AND target_remote_question_id IS NULL
   AND target_object_id IS NULL;

ALTER TABLE activities DROP CONSTRAINT target_not_null;
ALTER TABLE activities
  ADD CONSTRAINT target_not_null
  CHECK (
    NOT (
      target_note_id IS NULL
      AND target_remote_note_id IS NULL
      AND target_profile_id IS NULL
      AND target_remote_actor_id IS NULL
      AND target_activity_id IS NULL
      AND target_remote_question_id IS NULL
      AND target_object_id IS NULL
      AND target_actor_id IS NULL));
