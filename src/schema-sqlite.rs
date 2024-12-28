diesel::table! {
    activities (id) {
        id -> Integer,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        kind -> Text,
        uuid -> Text,
        actor -> Text,
        ap_to -> Nullable<Jsonb>,
        cc -> Nullable<Jsonb>,
        target_activity_id -> Nullable<Integer>,
        target_ap_id -> Nullable<Text>,
        revoked -> Bool,
        ap_id -> Nullable<Text>,
        reply -> Bool,
        raw -> Nullable<Jsonb>,
        target_object_id -> Nullable<Integer>,
        actor_id -> Nullable<Integer>,
        target_actor_id -> Nullable<Integer>,
        log -> Nullable<Jsonb>,
        instrument -> Nullable<Jsonb>,
    }
}

diesel::table! {
    actors (id) {
        id -> Int4,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        ek_uuid -> Nullable<Text>,
        ek_username -> Nullable<Text>,
        ek_summary_markdown -> Nullable<Text>,
        ek_avatar_filename -> Nullable<Text>,
        ek_banner_filename -> Nullable<Text>,
        ek_private_key -> Nullable<Text>,
        ek_password -> Nullable<Text>,
        ek_client_public_key -> Nullable<Text>,
        ek_client_private_key -> Nullable<Text>,
        ek_salt -> Nullable<Text>,
        ek_olm_pickled_account -> Nullable<Text>,
        ek_olm_pickled_account_hash -> Nullable<Text>,
        ek_olm_identity_key -> Nullable<Text>,
        ek_webfinger -> Nullable<Text>,
        ek_checked_at -> TimestamptzSqlite,
        ek_hashtags -> Jsonb,
        as_type -> Text,
        as_context -> Nullable<Jsonb>,
        as_id -> Text,
        as_name -> Nullable<Text>,
        as_preferred_username -> Nullable<Text>,
        as_summary -> Nullable<Text>,
        as_inbox -> Text,
        as_outbox -> Text,
        as_followers -> Nullable<Text>,
        as_following -> Nullable<Text>,
        as_liked -> Nullable<Text>,
        as_public_key -> Jsonb,
        as_featured -> Nullable<Text>,
        as_featured_tags -> Nullable<Text>,
        as_url -> Nullable<Jsonb>,
        as_published -> Nullable<TimestamptzSqlite>,
        as_tag -> Jsonb,
        as_attachment -> Jsonb,
        as_endpoints -> Jsonb,
        as_icon -> Jsonb,
        as_image -> Jsonb,
        as_also_known_as -> Jsonb,
        as_discoverable -> Bool,
        ap_capabilities -> Jsonb,
        ap_manually_approves_followers -> Bool,
        ek_keys -> Nullable<Text>,
        ek_last_decrypted_activity -> TimestamptzSqlite,
    }
}

diesel::table! {
    cache (id) {
        id -> Int4,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        uuid -> Text,
        url -> Text,
        media_type -> Nullable<Text>,
        height -> Nullable<Int4>,
        width -> Nullable<Int4>,
        blurhash -> Nullable<Text>,
    }
}

diesel::table! {
    followers (id) {
        id -> Int4,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        ap_id -> Text,
        actor -> Text,
        followed_ap_id -> Text,
        uuid -> Text,
        actor_id -> Int4,
    }
}

diesel::table! {
    instances (id) {
        id -> Int4,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        domain_name -> Text,
        json -> Nullable<Jsonb>,
        blocked -> Bool,
        last_message_at -> TimestamptzSqlite,
        shared_inbox -> Nullable<Text>,
    }
}

diesel::table! {
    leaders (id) {
        id -> Int4,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        actor -> Text,
        leader_ap_id -> Text,
        uuid -> Text,
        accept_ap_id -> Nullable<Text>,
        accepted -> Nullable<Bool>,
        follow_ap_id -> Nullable<Text>,
        actor_id -> Int4,
    }
}

diesel::table! {
    notifications (id) {
        id -> Int4,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        uuid -> Text,
        kind -> Text,
        profile_id -> Int4,
        activity_id -> Int4,
    }
}

diesel::table! {
    objects (id) {
        id -> Int4,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        ap_conversation -> Nullable<Text>,
        ap_sensitive -> Nullable<Bool>,
        ap_signature -> Nullable<Jsonb>,
        ap_voters_count -> Nullable<Int4>,
        as_any_of -> Nullable<Jsonb>,
        as_attachment -> Nullable<Jsonb>,
        as_attributed_to -> Nullable<Jsonb>,
        as_audience -> Nullable<Jsonb>,
        as_bcc -> Nullable<Jsonb>,
        as_bto -> Nullable<Jsonb>,
        as_cc -> Nullable<Jsonb>,
        as_closed -> Nullable<Jsonb>,
        as_content -> Nullable<Text>,
        as_content_map -> Nullable<Jsonb>,
        as_context -> Nullable<Jsonb>,
        as_deleted -> Nullable<TimestamptzSqlite>,
        as_describes -> Nullable<Jsonb>,
        as_duration -> Nullable<Text>,
        as_end_time -> Nullable<TimestamptzSqlite>,
        as_former_type -> Nullable<Text>,
        as_generator -> Nullable<Jsonb>,
        as_icon -> Nullable<Jsonb>,
        as_id -> Text,
        as_image -> Nullable<Jsonb>,
        as_in_reply_to -> Nullable<Jsonb>,
        as_location -> Nullable<Jsonb>,
        as_media_type -> Nullable<Text>,
        as_name -> Nullable<Text>,
        as_name_map -> Nullable<Jsonb>,
        as_one_of -> Nullable<Jsonb>,
        as_preview -> Nullable<Jsonb>,
        as_published -> Nullable<TimestamptzSqlite>,
        as_replies -> Nullable<Jsonb>,
        as_start_time -> Nullable<TimestamptzSqlite>,
        as_summary -> Nullable<Text>,
        as_summary_map -> Nullable<Jsonb>,
        as_tag -> Nullable<Jsonb>,
        as_to -> Nullable<Jsonb>,
        as_type -> Text,
        as_updated -> Nullable<TimestamptzSqlite>,
        as_url -> Nullable<Jsonb>,
        ek_hashtags -> Jsonb,
        ek_instrument -> Nullable<Jsonb>,
        ek_metadata -> Nullable<Jsonb>,
        ek_profile_id -> Nullable<Int4>,
        ek_uuid -> Nullable<Text>,
    }
}

diesel::table! {
    olm_one_time_keys (id) {
        id -> Int4,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        uuid -> Text,
        profile_id -> Int4,
        olm_id -> Int4,
        key_data -> Text,
        distributed -> Bool,
        assignee -> Nullable<Text>,
    }
}

diesel::table! {
    olm_sessions (id) {
        id -> Int4,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        uuid -> Text,
        session_data -> Text,
        session_hash -> Text,
        owner_as_id -> Text,
        ap_conversation -> Text,
        owner_id -> Int4,
    }
}

diesel::table! {
    unprocessable (id) {
        id -> Int4,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        raw -> Jsonb,
        error -> Nullable<Text>,
    }
}

diesel::table! {
    vault (id) {
        id -> Int4,
        created_at -> TimestamptzSqlite,
        updated_at -> TimestamptzSqlite,
        uuid -> Text,
        owner_as_id -> Text,
        activity_id -> Int4,
        data -> Text,
    }
}

diesel::joinable!(olm_one_time_keys -> actors (profile_id));
diesel::joinable!(vault -> activities (activity_id));

diesel::allow_tables_to_appear_in_same_query!(
    activities,
    actors,
    cache,
    followers,
    instances,
    leaders,
    notifications,
    objects,
    olm_one_time_keys,
    olm_sessions,
    unprocessable,
    vault,
);
