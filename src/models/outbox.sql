-- This query is optimized for public timelines WITHOUT hashtag filters.
WITH top_activities AS (
    SELECT id
    FROM (
        -- First, find the single most recent activity for EACH unique object that matches our filters.
        SELECT DISTINCT ON (a.target_ap_id)
            a.id,
            a.created_at
        FROM activities a
        JOIN objects o ON a.target_object_id = o.id
        JOIN actors ac ON a.actor = ac.as_id
        WHERE
            a.revoked = false
            AND a.kind IN ('create', 'announce')
            AND o.as_type IN ('note', 'question', 'article')
            AND (ap_to ?| $1 OR cc ?| $1) -- to_addresses (Uses GIN index)
            AND (CASE WHEN $2 <> 'NULL' THEN ac.ek_username = $2 ELSE TRUE END)
            AND (CASE WHEN $3 <> 'NULL' THEN a.created_at < $3::timestamptz ELSE TRUE END) -- max_date
            AND (CASE WHEN $4 <> 'NULL' THEN a.created_at > $4::timestamptz ELSE TRUE END) -- min_date
        -- This ordering is crucial for DISTINCT ON to pick the latest activity per object
        ORDER BY a.target_ap_id, a.created_at DESC
    ) AS latest_activities_per_object
     -- Now, sort the unique objects by their activity time and take the top N.
    ORDER BY created_at DESC
    -- Param 8: limit
    LIMIT $5
),
main AS (
    SELECT
        a.id,
        a.created_at,
        a.updated_at,
        a.kind,
        a.uuid,
        a.actor,
        a.ap_to,
        a.cc,
        a.target_activity_id,
        a.target_ap_id,
        a.revoked,
        a.ap_id,
        a.reply,
        a.target_object_id,
        a.actor_id,
        a.target_actor_id,
        a.instrument,
        a2.created_at AS recursive_created_at,
        a2.updated_at AS recursive_updated_at,
        a2.kind AS recursive_kind,
        a2.uuid AS recursive_uuid,
        a2.actor AS recursive_actor,
        a2.ap_to AS recursive_ap_to,
        a2.cc AS recursive_cc,
        a2.target_activity_id AS recursive_target_activity_id,
        a2.target_ap_id AS recursive_target_ap_id,
        a2.revoked AS recursive_revoked,
        a2.ap_id AS recursive_ap_id,
        a2.reply AS recursive_reply,
        a2.target_object_id AS recursive_target_object_id,
        a2.actor_id AS recursive_actor_id,
        a2.target_actor_id AS recursive_target_actor_id,
        a2.instrument AS recursive_instrument,
        COALESCE(o.created_at, o2.created_at) AS object_created_at,
        COALESCE(o.updated_at, o2.updated_at) AS object_updated_at,
        COALESCE(o.ek_uuid, o2.ek_uuid) AS object_uuid,
        COALESCE(o.as_type, o2.as_type) AS object_type,
        COALESCE(o.as_published, o2.as_published) AS object_published,
        COALESCE(o.as_id, o2.as_id) AS object_as_id,
        COALESCE(o.as_name, o2.as_name) AS object_name,
        COALESCE(o.as_url, o2.as_url) AS object_url,
        COALESCE(o.as_to, o2.as_to) AS object_to,
        COALESCE(o.as_cc, o2.as_cc) AS object_cc,
        COALESCE(o.as_tag, o2.as_tag) AS object_tag,
        COALESCE(o.as_attributed_to, o2.as_attributed_to) AS object_attributed_to,
        COALESCE(o.as_in_reply_to, o2.as_in_reply_to) AS object_in_reply_to,
        COALESCE(o.as_content, o2.as_content) AS object_content,
        COALESCE(o.ap_conversation, o2.ap_conversation) AS object_conversation,
        COALESCE(o.as_attachment, o2.as_attachment) AS object_attachment,
        COALESCE(o.as_summary, o2.as_summary) AS object_summary,
        COALESCE(o.as_preview, o2.as_preview) AS object_preview,
        COALESCE(o.as_start_time, o2.as_start_time) AS object_start_time,
        COALESCE(o.as_end_time, o2.as_end_time) AS object_end_time,
        COALESCE(o.as_one_of, o2.as_one_of) AS object_one_of,
        COALESCE(o.as_any_of, o2.as_any_of) AS object_any_of,
        COALESCE(o.ap_voters_count, o2.ap_voters_count) AS object_voters_count,
        COALESCE(o.ap_sensitive, o2.ap_sensitive) AS object_sensitive,
        COALESCE(o.ek_metadata, o2.ek_metadata) AS object_metadata,
        COALESCE(o.ek_profile_id, o2.ek_profile_id) AS object_profile_id,
        COALESCE(o.ek_instrument, o2.ek_instrument) AS object_instrument,
        COALESCE(ta.created_at, ta2.created_at) AS actor_created_at,
        COALESCE(ta.updated_at, ta2.updated_at) AS actor_updated_at,
        COALESCE(ta.ek_uuid, ta2.ek_uuid) AS actor_uuid,
        COALESCE(ta.ek_username, ta2.ek_username) AS actor_username,
        COALESCE(ta.ek_summary_markdown, ta2.ek_summary_markdown) AS actor_summary_markdown,
        COALESCE(ta.ek_avatar_filename, ta2.ek_avatar_filename) AS actor_avatar_filename,
        COALESCE(ta.ek_banner_filename, ta2.ek_banner_filename) AS actor_banner_filename,
        COALESCE(ta.ek_private_key, ta2.ek_private_key) AS actor_private_key,
        COALESCE(ta.ek_password, ta2.ek_password) AS actor_password,
        COALESCE(ta.ek_client_public_key, ta2.ek_client_public_key) AS actor_client_public_key,
        COALESCE(ta.ek_client_private_key, ta2.ek_client_private_key) AS actor_client_private_key,
        COALESCE(ta.ek_salt, ta2.ek_salt) AS actor_salt,
        COALESCE(ta.ek_olm_pickled_account, ta2.ek_olm_pickled_account) AS actor_olm_pickled_account,
        COALESCE(ta.ek_olm_pickled_account_hash, ta2.ek_olm_pickled_account_hash) AS actor_olm_pickled_account_hash,
        COALESCE(ta.ek_olm_identity_key, ta2.ek_olm_identity_key) AS actor_olm_identity_key,
        COALESCE(ta.ek_webfinger, ta2.ek_webfinger) AS actor_webfinger,
        COALESCE(ta.ek_checked_at, ta2.ek_checked_at) AS actor_checked_at,
        COALESCE(ta.ek_hashtags, ta2.ek_hashtags) AS actor_hashtags,
        COALESCE(ta.as_type, ta2.as_type) AS actor_type,
        COALESCE(ta.as_context, ta2.as_context) AS actor_context,
        COALESCE(ta.as_id, ta2.as_id) AS actor_as_id,
        COALESCE(ta.as_name, ta2.as_name) AS actor_name,
        COALESCE(ta.as_preferred_username, ta2.as_preferred_username) AS actor_preferred_username,
        COALESCE(ta.as_summary, ta2.as_summary) AS actor_summary,
        COALESCE(ta.as_inbox, ta2.as_inbox) AS actor_inbox,
        COALESCE(ta.as_outbox, ta2.as_outbox) AS actor_outbox,
        COALESCE(ta.as_followers, ta2.as_followers) AS actor_followers,
        COALESCE(ta.as_following, ta2.as_following) AS actor_following,
        COALESCE(ta.as_liked, ta2.as_liked) AS actor_liked,
        COALESCE(ta.as_public_key, ta2.as_public_key) AS actor_public_key,
        COALESCE(ta.as_featured, ta2.as_featured) AS actor_featured,
        COALESCE(ta.as_featured_tags, ta2.as_featured_tags) AS actor_featured_tags,
        COALESCE(ta.as_url, ta2.as_url) AS actor_url,
        COALESCE(ta.as_published, ta2.as_published) AS actor_published,
        COALESCE(ta.as_tag, ta2.as_tag) AS actor_tag,
        COALESCE(ta.as_attachment, ta2.as_attachment) AS actor_attachment,
        COALESCE(ta.as_endpoints, ta2.as_endpoints) AS actor_endpoints,
        COALESCE(ta.as_icon, ta2.as_icon) AS actor_icon,
        COALESCE(ta.as_image, ta2.as_image) AS actor_image,
        COALESCE(ta.as_also_known_as, ta2.as_also_known_as) AS actor_also_known_as,
        COALESCE(ta.as_discoverable, ta2.as_discoverable) AS actor_discoverable,
        COALESCE(ta.ap_capabilities, ta2.ap_capabilities) AS actor_capabilities,
        COALESCE(ta.ek_keys, ta2.ek_keys) AS actor_keys,
        COALESCE(ta.ek_last_decrypted_activity, ta2.ek_last_decrypted_activity) AS actor_last_decrypted_activity,
        COALESCE(ta.ap_manually_approves_followers, ta2.ap_manually_approves_followers) AS actor_manually_approves_followers,
        COALESCE(ta.ek_mls_credentials, ta2.ek_mls_credentials) AS actor_mls_credentials,
        COALESCE(ta.ek_mls_storage, ta2.ek_mls_storage) AS actor_mls_storage,
        COALESCE(ta.ek_mls_storage_hash, ta2.ek_mls_storage_hash) AS actor_mls_storage_hash,
        COALESCE(ta.ek_muted_terms, ta2.ek_muted_terms) AS actor_muted_terms
    FROM activities a
         JOIN top_activities ta_filter ON a.id = ta_filter.id
        LEFT JOIN objects o ON (o.id = a.target_object_id)
        LEFT JOIN actors ta ON (ta.id = a.target_actor_id)
        LEFT JOIN activities a2 ON (a.target_activity_id = a2.id)
        LEFT JOIN objects o2 ON (a2.target_object_id = o2.id)
        LEFT JOIN actors ta2 ON (ta2.id = a2.target_actor_id)
),
announced AS (
    SELECT
        m.id,
        a.ap_id AS object_announced
    FROM main m
    LEFT JOIN activities a ON (
        a.target_ap_id = m.object_as_id
        AND NOT a.revoked
        AND a.kind = 'announce'
        AND (CASE WHEN $6 <> 'NULL' THEN a.actor_id = $6::integer ELSE FALSE END)
    )
    GROUP BY m.id, a.ap_id
),
liked AS (
    SELECT
        m.id,
        a.ap_id AS object_liked
    FROM main m
    LEFT JOIN activities a ON (
        a.target_ap_id = m.object_as_id
        AND NOT a.revoked
        AND a.kind = 'like'
        AND (CASE WHEN $6 <> 'NULL' THEN a.actor_id = $6::integer ELSE FALSE END)
    )
    GROUP BY m.id, a.ap_id
),
attributed_actors AS (
    WITH unnested_ids AS (
        -- First, extract actor IDs from the string case for our 20 main activities
        SELECT m.id as main_id, m.object_attributed_to ->> 0 as actor_as_id
        FROM main m
        WHERE jsonb_typeof(m.object_attributed_to) = 'string'
        
        UNION ALL
        
        -- Then, extract actor IDs from the array case for our 20 main activities
        SELECT m.id as main_id, jsonb_array_elements_text(m.object_attributed_to) as actor_as_id
        FROM main m
        WHERE jsonb_typeof(m.object_attributed_to) = 'array'
    )
    SELECT
        u.main_id,
        JSONB_AGG(jsonb_build_object(
            'id', ac.as_id, 'name', ac.as_name, 'tag', ac.as_tag, 'url', ac.as_url,
            'icon', ac.as_icon, 'preferredUsername', ac.as_preferred_username, 'webfinger', ac.ek_webfinger
        ) ORDER BY ac.as_id) AS attributed_profiles
    FROM unnested_ids u
    -- This JOIN is now simple and will use the index on actors.as_id
    JOIN actors ac ON u.actor_as_id = ac.as_id
    GROUP BY u.main_id
)
SELECT
    m.*,
    COALESCE(JSONB_AGG(jsonb_build_object('id', ac.as_id, 'name', ac.as_name, 'tag', ac.as_tag, 'url', ac.as_url, 'icon', ac.as_icon, 'preferredUsername', ac.as_preferred_username)) FILTER (WHERE a.actor IS NOT NULL
            AND a.kind = 'announce'), '[]') AS object_announcers,
    COALESCE(JSONB_AGG(jsonb_build_object('id', ac.as_id, 'name', ac.as_name, 'tag', ac.as_tag, 'url', ac.as_url, 'icon', ac.as_icon, 'preferredUsername', ac.as_preferred_username)) FILTER (WHERE a.actor IS NOT NULL
            AND a.kind = 'like'), '[]') AS object_likers,
    COALESCE(aa.attributed_profiles, '[]') AS object_attributed_to_profiles,
    announced.object_announced,
    liked.object_liked
FROM
    main m
    LEFT JOIN activities a ON (a.target_ap_id = m.object_as_id
            AND NOT a.revoked
            AND (a.kind = 'announce' OR a.kind = 'like'))
    LEFT JOIN actors ac ON (ac.as_id = a.actor)
    LEFT JOIN announced ON m.id = announced.id
    LEFT JOIN liked ON m.id = liked.id
    LEFT JOIN attributed_actors aa ON m.id = aa.main_id
GROUP BY
    m.id, m.created_at, m.updated_at, m.kind, m.uuid, m.actor, m.ap_to, m.cc, m.target_activity_id,
    m.target_ap_id, m.revoked, m.ap_id, m.reply, m.instrument, m.recursive_created_at, m.recursive_updated_at,
    m.recursive_kind, m.recursive_uuid, m.recursive_actor, m.recursive_ap_to, m.recursive_cc,
    m.recursive_target_activity_id, m.recursive_target_ap_id, m.recursive_revoked, m.recursive_ap_id,
    m.recursive_reply, m.recursive_target_object_id, m.recursive_actor_id, m.recursive_target_actor_id,
    m.recursive_instrument, m.object_created_at, m.object_updated_at, m.object_uuid, m.object_type,
    m.object_published, m.object_as_id, m.object_name, m.object_url, m.object_to, m.object_cc, m.object_tag,
    m.object_attributed_to, m.object_in_reply_to, m.object_content, m.object_conversation, m.object_attachment,
    m.object_summary, m.object_preview, m.object_start_time, m.object_end_time, m.object_one_of,
    m.object_any_of, m.object_voters_count, m.object_sensitive, m.object_metadata, m.object_profile_id,
    m.object_instrument, m.actor_id, m.target_actor_id, m.target_object_id, m.actor_created_at,
    m.actor_updated_at, m.actor_uuid, m.actor_username, m.actor_summary_markdown, m.actor_avatar_filename,
    m.actor_banner_filename, m.actor_private_key, m.actor_password, m.actor_client_public_key,
    m.actor_client_private_key, m.actor_salt, m.actor_olm_pickled_account, m.actor_olm_pickled_account_hash,
    m.actor_olm_identity_key, m.actor_webfinger, m.actor_checked_at, m.actor_hashtags, m.actor_type,
    m.actor_context, m.actor_as_id, m.actor_name, m.actor_preferred_username, m.actor_summary,
    m.actor_inbox, m.actor_outbox, m.actor_followers, m.actor_following, m.actor_liked, m.actor_public_key,
    m.actor_featured, m.actor_featured_tags, m.actor_url, m.actor_published, m.actor_tag,
    m.actor_attachment, m.actor_endpoints, m.actor_icon, m.actor_image, m.actor_also_known_as,
    m.actor_discoverable, m.actor_capabilities, m.actor_keys, m.actor_last_decrypted_activity,
    m.actor_manually_approves_followers, m.actor_mls_credentials, m.actor_mls_storage, m.actor_mls_storage_hash,
    m.actor_muted_terms,
    announced.object_announced,
    liked.object_liked,
    aa.attributed_profiles
ORDER BY
    CASE WHEN $7::boolean THEN m.created_at END ASC,
    CASE WHEN NOT $7::boolean THEN m.created_at END DESC;

-- 
-- PARAMETER ORDER:
-- 1: to_addresses (Text[])
-- 2: username (Text)
-- 3: max_date (Text)
-- 4: min_date (Text)
-- 5: limit (Integer)
-- 6: profile_actor_id (Text)
-- 7: order_asc (Boolean)

-- Example 1: Global Timeline (Unauthenticated)
-- \bind '{"https://www.w3.org/ns/activitystreams#Public","as:Public","Public"}' 'jdt' 'NULL' 'NULL' 20 'NULL' FALSE
-- \g
