--EXPLAIN ANALYZE
WITH main AS (
    SELECT DISTINCT ON (c.depth,
        o.as_id)
        - 1 AS id,
        o.created_at AS created_at,
        o.updated_at AS updated_at,
        'create' AS kind,
        '0' AS uuid,
        CASE WHEN jsonb_typeof(o.as_attributed_to) = 'string' THEN
            trim(BOTH '"' FROM o.as_attributed_to::text)
        WHEN jsonb_typeof(o.as_attributed_to) = 'array' THEN
            o.as_attributed_to ->> 0
        ELSE
            ''
        END AS actor,
        NULL::jsonb AS ap_to,
        NULL::jsonb AS cc,
        NULL::integer AS target_activity_id,
        o.as_id AS target_ap_id,
        FALSE AS revoked,
        o.as_id || '#synthesized-activity' AS ap_id,
        CASE WHEN o.as_in_reply_to IS NOT NULL THEN
            TRUE
        ELSE
            NULL
        END AS reply,
        NULL::jsonb AS raw,
        o.id AS target_object_id,
        NULL::integer AS actor_id,
        NULL::integer AS target_actor_id,
        NULL::jsonb AS log,
        NULL::jsonb AS instrument,
        NULL::timestamptz AS as_published,
        NULL::timestamp AS recursive_created_at,
        NULL::timestamp AS recursive_updated_at,
        NULL::activity_type AS recursive_kind,
        NULL AS recursive_uuid,
        NULL AS recursive_actor,
        NULL::jsonb AS recursive_ap_to,
        NULL::jsonb AS recursive_cc,
        NULL::integer AS recursive_target_activity_id,
        NULL AS recursive_target_ap_id,
        NULL::boolean AS recursive_revoked,
        NULL AS recursive_ap_id,
        NULL::boolean AS recursive_reply,
        NULL::integer AS recursive_target_object_id,
        NULL::integer AS recursive_actor_id,
        NULL::integer AS recursive_target_actor_id,
        NULL::jsonb AS recursive_instrument,
        o.created_at AS object_created_at,
        o.updated_at AS object_updated_at,
        o.ek_uuid AS object_uuid,
        o.as_type AS object_type,
        o.as_published AS object_published,
        o.as_updated AS object_updated,
        o.as_id AS object_as_id,
        o.as_name AS object_name,
        o.as_url AS object_url,
        o.as_to AS object_to,
        o.as_cc AS object_cc,
        o.as_tag AS object_tag,
        o.as_attributed_to AS object_attributed_to,
        o.as_in_reply_to AS object_in_reply_to,
        o.as_content AS object_content,
        o.ap_conversation AS object_conversation,
        o.as_attachment AS object_attachment,
        o.as_summary AS object_summary,
        o.as_preview AS object_preview,
        o.as_start_time AS object_start_time,
        o.as_end_time AS object_end_time,
        o.as_one_of AS object_one_of,
        o.as_any_of AS object_any_of,
        o.ap_voters_count AS object_voters_count,
        o.ap_sensitive AS object_sensitive,
        o.ek_metadata AS object_metadata,
        o.ek_profile_id AS object_profile_id,
        o.ek_instrument AS object_instrument,
        o.ap_source AS object_source,
        NULL::timestamp AS actor_created_at,
        NULL::timestamp AS actor_updated_at,
        NULL AS actor_uuid,
        NULL AS actor_username,
        NULL AS actor_summary_markdown,
        NULL AS actor_avatar_filename,
        NULL AS actor_banner_filename,
        NULL::jsonb AS actor_private_key,
        NULL AS actor_password,
        NULL AS actor_client_public_key,
        NULL AS actor_client_private_key,
        NULL AS actor_salt,
        NULL AS actor_olm_pickled_account,
        NULL AS actor_olm_pickled_account_hash,
        NULL AS actor_olm_identity_key,
        NULL AS actor_webfinger,
        NULL::timestamp AS actor_checked_at,
        NULL::jsonb AS actor_hashtags,
        NULL::actor_type AS actor_type,
        NULL::jsonb AS actor_context,
        NULL AS actor_as_id,
        NULL AS actor_name,
        NULL AS actor_preferred_username,
        NULL AS actor_summary,
        NULL AS actor_inbox,
        NULL AS actor_outbox,
        NULL AS actor_followers,
        NULL AS actor_following,
        NULL AS actor_liked,
        NULL::jsonb AS actor_public_key,
        NULL AS actor_featured,
        NULL AS actor_featured_tags,
        NULL::jsonb AS actor_url,
        NULL::timestamp AS actor_published,
        NULL::jsonb AS actor_tag,
        NULL::jsonb AS actor_attachment,
        NULL::jsonb AS actor_endpoints,
        NULL::jsonb AS actor_icon,
        NULL::jsonb AS actor_image,
        NULL::jsonb AS actor_also_known_as,
        NULL::boolean AS actor_discoverable,
        NULL::jsonb AS actor_capabilities,
        NULL AS actor_keys,
        NULL::timestamp AS actor_last_decrypted_activity,
        NULL::boolean AS actor_manually_approves_followers,
        NULL AS actor_mls_credentials,
        NULL AS actor_mls_storage,
        NULL AS actor_mls_storage_hash,
        NULL::jsonb AS actor_muted_terms
    FROM
        objects_closure c
        LEFT JOIN objects o ON (($4::boolean = TRUE
                    AND o.as_id = c.descendant)
                OR ($5::boolean = TRUE
                    AND o.as_id = c.ancestor))
        LEFT JOIN activities a ON a.target_ap_id = o.as_id
    WHERE (($4::boolean = TRUE
            AND c.ancestor = $1
            AND c.depth > 0) -- descendants
        OR ($5::boolean = TRUE
            AND c.descendant = $1
            AND c.depth > 0)) -- ancestors
    AND (o.as_type = 'note' OR o.as_type = 'question' OR o.as_type = 'article')
ORDER BY
    c.depth,
    o.as_id
),
announced AS (
    SELECT
        m.id,
        CASE WHEN $3::boolean = TRUE THEN
            a.ap_id
        ELSE
            NULL
        END AS object_announced
    FROM
        main m
        LEFT JOIN activities a ON (a.target_ap_id = m.object_as_id
                AND NOT a.revoked
                AND a.kind = 'announce'
                AND a.actor_id = $2)
    GROUP BY
        m.id,
        a.ap_id
),
liked AS (
    SELECT
        m.id,
        CASE WHEN $3::boolean = TRUE THEN
            a.ap_id
        ELSE
            NULL
        END AS object_liked
    FROM
        main m
        LEFT JOIN activities a ON (a.target_ap_id = m.object_as_id
                AND NOT a.revoked
                AND a.kind = 'like'
                AND a.actor_id = $2)
    GROUP BY
        m.id,
        a.ap_id
)
SELECT
    m.*,
    COALESCE(JSONB_AGG(jsonb_build_object('id', ac.as_id, 'name', ac.as_name, 'tag', ac.as_tag, 'url', ac.as_url, 'icon', ac.as_icon, 'preferredUsername', ac.as_preferred_username, 'webfinger', ac.ek_webfinger)) FILTER (WHERE a.actor IS NOT NULL
            AND a.kind = 'announce'), '[]') AS object_announcers,
    COALESCE(JSONB_AGG(jsonb_build_object('id', ac.as_id, 'name', ac.as_name, 'tag', ac.as_tag, 'url', ac.as_url, 'icon', ac.as_icon, 'preferredUsername', ac.as_preferred_username, 'webfinger', ac.ek_webfinger)) FILTER (WHERE a.actor IS NOT NULL
            AND a.kind = 'like'), '[]') AS object_likers,
COALESCE((
    SELECT JSONB_AGG(jsonb_build_object('id', ac2.as_id, 'name', ac2.as_name, 'tag', ac2.as_tag, 'url', ac2.as_url, 'icon', ac2.as_icon, 'preferredUsername', ac2.as_preferred_username, 'webfinger', ac2.ek_webfinger))
    FROM (
        SELECT DISTINCT attr_id
        FROM (
            -- Extract attributed_to values for this specific row
            SELECT 
                CASE 
                    WHEN jsonb_typeof(m.object_attributed_to) = 'string' THEN
                        m.object_attributed_to #>> '{}'
                    ELSE NULL
                END AS attr_id
            WHERE jsonb_typeof(m.object_attributed_to) = 'string'
            
            UNION ALL
            
            SELECT jsonb_array_elements_text(m.object_attributed_to) AS attr_id
            WHERE jsonb_typeof(m.object_attributed_to) = 'array'
        ) AS attr_values
        WHERE attr_id IS NOT NULL
    ) AS distinct_attrs
    JOIN actors ac2 ON ac2.as_id = distinct_attrs.attr_id
), '[]') AS object_attributed_to_profiles,
    announced.object_announced,
    liked.object_liked,
    NULL AS vault_id,
    NULL AS vault_created_at,
    NULL AS vault_updated_at,
    NULL AS vault_uuid,
    NULL AS vault_owner_as_id,
    NULL::int AS vault_activity_id,
    NULL AS vault_data,
    NULL AS mls_group_id_id,
    NULL AS mls_group_id_created_at,
    NULL AS mls_group_id_updated_at,
    NULL AS mls_group_id_uuid,
    NULL AS mls_group_id_actor_id,
    NULL AS mls_group_id_conversation,
    NULL AS mls_group_id_mls_group
FROM
    main m
    LEFT JOIN activities a ON (a.target_ap_id = m.object_as_id
            AND NOT a.revoked
            AND (a.kind = 'announce'
                OR a.kind = 'like'))
    LEFT JOIN actors ac ON (ac.as_id = a.actor)
    LEFT JOIN announced ON m.id = announced.id
    LEFT JOIN liked ON m.id = liked.id
GROUP BY
    m.id,
    m.created_at,
    m.updated_at,
    m.kind,
    m.uuid,
    m.actor,
    m.ap_to,
    m.cc,
    m.target_activity_id,
    m.target_ap_id,
    m.revoked,
    m.ap_id,
    m.reply,
    m.raw,
    m.target_object_id,
    m.actor_id,
    m.target_actor_id,
    m.log,
    m.instrument,
    m.as_published,
    m.recursive_created_at,
    m.recursive_updated_at,
    m.recursive_kind,
    m.recursive_uuid,
    m.recursive_actor,
    m.recursive_ap_to,
    m.recursive_cc,
    m.recursive_target_activity_id,
    m.recursive_target_ap_id,
    m.recursive_revoked,
    m.recursive_ap_id,
    m.recursive_reply,
    m.recursive_target_object_id,
    m.recursive_actor_id,
    m.recursive_target_actor_id,
    m.recursive_instrument,
    m.object_created_at,
    m.object_updated_at,
    m.object_uuid,
    m.object_type,
    m.object_published,
    m.object_updated,
    m.object_as_id,
    m.object_name,
    m.object_url,
    m.object_to,
    m.object_cc,
    m.object_tag,
    m.object_attributed_to,
    m.object_content,
    m.object_conversation,
    m.object_attachment,
    m.object_summary,
    m.object_preview,
    m.object_start_time,
    m.object_end_time,
    m.object_one_of,
    m.object_any_of,
    m.object_voters_count,
    m.object_sensitive,
    m.object_metadata,
    m.object_profile_id,
    m.object_in_reply_to,
    m.object_instrument,
    m.object_source,
    m.actor_created_at,
    m.actor_updated_at,
    m.actor_uuid,
    m.actor_username,
    m.actor_summary_markdown,
    m.actor_avatar_filename,
    m.actor_banner_filename,
    m.actor_private_key,
    m.actor_password,
    m.actor_client_public_key,
    m.actor_client_private_key,
    m.actor_salt,
    m.actor_olm_pickled_account,
    m.actor_olm_pickled_account_hash,
    m.actor_olm_identity_key,
    m.actor_webfinger,
    m.actor_checked_at,
    m.actor_hashtags,
    m.actor_type,
    m.actor_context,
    m.actor_as_id,
    m.actor_name,
    m.actor_preferred_username,
    m.actor_summary,
    m.actor_inbox,
    m.actor_outbox,
    m.actor_followers,
    m.actor_following,
    m.actor_liked,
    m.actor_public_key,
    m.actor_featured,
    m.actor_featured_tags,
    m.actor_url,
    m.actor_published,
    m.actor_tag,
    m.actor_attachment,
    m.actor_endpoints,
    m.actor_icon,
    m.actor_image,
    m.actor_also_known_as,
    m.actor_discoverable,
    m.actor_capabilities,
    m.actor_keys,
    m.actor_last_decrypted_activity,
    m.actor_manually_approves_followers,
    m.actor_mls_credentials,
    m.actor_mls_storage,
    m.actor_mls_storage_hash,
    m.actor_muted_terms,
    announced.object_announced,
    liked.object_liked,
    vault_id,
    vault_created_at,
    vault_updated_at,
    vault_uuid,
    vault_owner_as_id,
    vault_activity_id,
    vault_data,
    mls_group_id_id,
    mls_group_id_created_at,
    mls_group_id_updated_at,
    mls_group_id_uuid,
    mls_group_id_actor_id,
    mls_group_id_conversation,
    mls_group_id_mls_group
ORDER BY
    created_at ASC;

-- created_at ASC
-- \bind 'https://mastodon.social/users/belldotbz/statuses/114754909171422185' 7 FALSE TRUE FALSE
-- \g

