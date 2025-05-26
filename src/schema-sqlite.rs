// @generated automatically by Diesel CLI.

diesel::table! {
    activities (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        kind -> Text,
        uuid -> Text,
        actor -> Text,
        ap_to -> Nullable<Jsonb>,
        cc -> Nullable<Jsonb>,
        target_activity_id -> Nullable<Integer>,
        target_ap_id -> Nullable<Text>,
        revoked -> Integer,
        ap_id -> Nullable<Text>,
        reply -> Integer,
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
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
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
        ek_checked_at -> Text,
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
        as_published -> Nullable<Text>,
        as_tag -> Jsonb,
        as_attachment -> Jsonb,
        as_endpoints -> Jsonb,
        as_icon -> Jsonb,
        as_image -> Jsonb,
        as_also_known_as -> Jsonb,
        as_discoverable -> Integer,
        ap_capabilities -> Jsonb,
        ap_manually_approves_followers -> Integer,
        ek_keys -> Nullable<Text>,
        ek_last_decrypted_activity -> Text,
        ek_mls_credentials -> Nullable<Text>,
        ek_mls_storage -> Nullable<Text>,
        ek_mls_storage_hash -> Nullable<Text>,
    }
}

diesel::table! {
    cache (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        uuid -> Text,
        url -> Text,
        media_type -> Nullable<Text>,
        height -> Nullable<Integer>,
        width -> Nullable<Integer>,
        blurhash -> Nullable<Text>,
        path -> Nullable<Text>,
    }
}

diesel::table! {
    encrypted_sessions (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        profile_id -> Integer,
        ap_to -> Text,
        attributed_to -> Text,
        instrument -> Jsonb,
        reference -> Nullable<Text>,
        uuid -> Text,
    }
}

diesel::table! {
    followers (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        ap_id -> Text,
        actor -> Text,
        followed_ap_id -> Text,
        uuid -> Text,
        actor_id -> Integer,
    }
}

diesel::table! {
    follows (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        uuid -> Text,
        profile_id -> Nullable<Integer>,
        ap_object -> Text,
        actor -> Text,
    }
}

diesel::table! {
    hashtag_trend (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        period -> Integer,
        hashtag -> Text,
        update_count -> Integer,
    }
}

diesel::table! {
    instances (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        domain_name -> Text,
        json -> Nullable<Jsonb>,
        blocked -> Integer,
        last_message_at -> Text,
        shared_inbox -> Nullable<Text>,
    }
}

diesel::table! {
    leaders (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        actor -> Text,
        leader_ap_id -> Text,
        uuid -> Text,
        accept_ap_id -> Nullable<Text>,
        accepted -> Nullable<Integer>,
        follow_ap_id -> Nullable<Text>,
        actor_id -> Integer,
    }
}

diesel::table! {
    mls_group_conversations (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        uuid -> Text,
        actor_id -> Integer,
        conversation -> Text,
        mls_group -> Text,
    }
}

diesel::table! {
    mls_key_packages (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        uuid -> Text,
        actor_id -> Integer,
        key_data -> Text,
        distributed -> Integer,
        assignee -> Nullable<Text>,
    }
}

diesel::table! {
    notifications (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        uuid -> Text,
        kind -> Text,
        profile_id -> Integer,
        activity_id -> Integer,
    }
}

diesel::table! {
    objects (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        ap_conversation -> Nullable<Text>,
        ap_sensitive -> Nullable<Integer>,
        ap_signature -> Nullable<Jsonb>,
        ap_voters_count -> Nullable<Integer>,
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
        as_deleted -> Nullable<Text>,
        as_describes -> Nullable<Jsonb>,
        as_duration -> Nullable<Text>,
        as_end_time -> Nullable<Text>,
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
        as_published -> Nullable<Text>,
        as_replies -> Nullable<Jsonb>,
        as_start_time -> Nullable<Text>,
        as_summary -> Nullable<Text>,
        as_summary_map -> Nullable<Jsonb>,
        as_tag -> Nullable<Jsonb>,
        as_to -> Nullable<Jsonb>,
        as_type -> Text,
        as_updated -> Nullable<Text>,
        as_url -> Nullable<Jsonb>,
        ek_hashtags -> Jsonb,
        ek_instrument -> Nullable<Jsonb>,
        ek_metadata -> Nullable<Jsonb>,
        ek_profile_id -> Nullable<Integer>,
        ek_uuid -> Nullable<Text>,
    }
}

diesel::table! {
    olm_one_time_keys (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        uuid -> Text,
        profile_id -> Integer,
        olm_id -> Integer,
        key_data -> Text,
        distributed -> Integer,
        assignee -> Nullable<Text>,
    }
}

diesel::table! {
    olm_sessions (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        uuid -> Text,
        session_data -> Text,
        session_hash -> Text,
        owner_as_id -> Text,
        ap_conversation -> Text,
        owner_id -> Integer,
    }
}

diesel::table! {
    processing_queue (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        profile_id -> Integer,
        ap_id -> Text,
        ap_to -> Jsonb,
        cc -> Nullable<Jsonb>,
        attributed_to -> Text,
        kind -> Text,
        ap_object -> Jsonb,
        processed -> Integer,
    }
}

diesel::table! {
    remote_encrypted_sessions (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        profile_id -> Integer,
        actor -> Text,
        kind -> Text,
        ap_id -> Text,
        ap_to -> Text,
        attributed_to -> Text,
        instrument -> Jsonb,
        reference -> Nullable<Text>,
    }
}

diesel::table! {
    unprocessable (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        raw -> Jsonb,
        error -> Nullable<Text>,
    }
}

diesel::table! {
    vault (id) {
        id -> Nullable<Integer>,
        created_at -> Text,
        updated_at -> Text,
        uuid -> Text,
        owner_as_id -> Text,
        activity_id -> Integer,
        data -> Text,
    }
}

diesel::joinable!(encrypted_sessions -> actors (profile_id));
diesel::joinable!(followers -> actors (actor_id));
diesel::joinable!(follows -> actors (profile_id));
diesel::joinable!(leaders -> actors (actor_id));
diesel::joinable!(mls_group_conversations -> actors (actor_id));
diesel::joinable!(mls_key_packages -> actors (actor_id));
diesel::joinable!(notifications -> activities (activity_id));
diesel::joinable!(notifications -> actors (profile_id));
diesel::joinable!(objects -> actors (ek_profile_id));
diesel::joinable!(olm_one_time_keys -> actors (profile_id));
diesel::joinable!(processing_queue -> actors (profile_id));
diesel::joinable!(remote_encrypted_sessions -> actors (profile_id));
diesel::joinable!(vault -> activities (activity_id));

diesel::allow_tables_to_appear_in_same_query!(
    activities,
    actors,
    cache,
    encrypted_sessions,
    followers,
    follows,
    hashtag_trend,
    instances,
    leaders,
    mls_group_conversations,
    mls_key_packages,
    notifications,
    objects,
    olm_one_time_keys,
    olm_sessions,
    processing_queue,
    remote_encrypted_sessions,
    unprocessable,
    vault,
);
