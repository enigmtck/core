// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
    #[diesel(postgres_type(name = "activity_type"))]
    pub struct ActivityType;

    #[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
    #[diesel(postgres_type(name = "actor_type"))]
    pub struct ActorType;

    #[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
    #[diesel(postgres_type(name = "notification_type"))]
    pub struct NotificationType;

    #[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
    #[diesel(postgres_type(name = "object_type"))]
    pub struct ObjectType;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ActivityType;

    activities (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        kind -> ActivityType,
        uuid -> Varchar,
        actor -> Varchar,
        ap_to -> Nullable<Jsonb>,
        cc -> Nullable<Jsonb>,
        target_activity_id -> Nullable<Int4>,
        target_ap_id -> Nullable<Varchar>,
        revoked -> Bool,
        ap_id -> Nullable<Varchar>,
        reply -> Bool,
        raw -> Nullable<Jsonb>,
        target_object_id -> Nullable<Int4>,
        actor_id -> Nullable<Int4>,
        target_actor_id -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ActorType;

    actors (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        ek_uuid -> Nullable<Varchar>,
        ek_username -> Nullable<Varchar>,
        ek_summary_markdown -> Nullable<Varchar>,
        ek_avatar_filename -> Nullable<Varchar>,
        ek_banner_filename -> Nullable<Varchar>,
        ek_private_key -> Nullable<Varchar>,
        ek_password -> Nullable<Varchar>,
        ek_client_public_key -> Nullable<Varchar>,
        ek_client_private_key -> Nullable<Varchar>,
        ek_salt -> Nullable<Varchar>,
        ek_olm_pickled_account -> Nullable<Varchar>,
        ek_olm_pickled_account_hash -> Nullable<Varchar>,
        ek_olm_identity_key -> Nullable<Varchar>,
        ek_webfinger -> Nullable<Varchar>,
        ek_checked_at -> Timestamptz,
        ek_hashtags -> Jsonb,
        as_type -> ActorType,
        as_context -> Nullable<Jsonb>,
        as_id -> Varchar,
        as_name -> Nullable<Varchar>,
        as_preferred_username -> Nullable<Varchar>,
        as_summary -> Nullable<Varchar>,
        as_inbox -> Varchar,
        as_outbox -> Varchar,
        as_followers -> Nullable<Varchar>,
        as_following -> Nullable<Varchar>,
        as_liked -> Nullable<Varchar>,
        as_public_key -> Jsonb,
        as_featured -> Nullable<Varchar>,
        as_featured_tags -> Nullable<Varchar>,
        as_url -> Nullable<Varchar>,
        as_published -> Nullable<Timestamptz>,
        as_tag -> Jsonb,
        as_attachment -> Jsonb,
        as_endpoints -> Jsonb,
        as_icon -> Jsonb,
        as_image -> Jsonb,
        as_also_known_as -> Jsonb,
        as_discoverable -> Bool,
        ap_capabilities -> Jsonb,
        ap_manually_approves_followers -> Bool,
    }
}

diesel::table! {
    cache (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        uuid -> Varchar,
        url -> Varchar,
        media_type -> Nullable<Varchar>,
        height -> Nullable<Int4>,
        width -> Nullable<Int4>,
        blurhash -> Nullable<Varchar>,
    }
}

diesel::table! {
    encrypted_sessions (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        profile_id -> Int4,
        ap_to -> Varchar,
        attributed_to -> Varchar,
        instrument -> Jsonb,
        reference -> Nullable<Varchar>,
        uuid -> Varchar,
    }
}

diesel::table! {
    followers (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        ap_id -> Varchar,
        actor -> Varchar,
        followed_ap_id -> Varchar,
        uuid -> Varchar,
        actor_id -> Int4,
    }
}

diesel::table! {
    follows (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        uuid -> Varchar,
        profile_id -> Nullable<Int4>,
        ap_object -> Varchar,
        actor -> Varchar,
    }
}

diesel::table! {
    hashtag_trend (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        period -> Int4,
        hashtag -> Varchar,
        update_count -> Int4,
    }
}

diesel::table! {
    instances (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        domain_name -> Varchar,
        json -> Nullable<Jsonb>,
        blocked -> Bool,
        last_message_at -> Timestamptz,
    }
}

diesel::table! {
    leaders (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        actor -> Varchar,
        leader_ap_id -> Varchar,
        uuid -> Varchar,
        accept_ap_id -> Nullable<Varchar>,
        accepted -> Nullable<Bool>,
        follow_ap_id -> Nullable<Varchar>,
        actor_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::NotificationType;

    notifications (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        uuid -> Varchar,
        kind -> NotificationType,
        profile_id -> Int4,
        activity_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ObjectType;

    objects (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
        as_deleted -> Nullable<Timestamptz>,
        as_describes -> Nullable<Jsonb>,
        as_duration -> Nullable<Text>,
        as_end_time -> Nullable<Timestamptz>,
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
        as_published -> Nullable<Timestamptz>,
        as_replies -> Nullable<Jsonb>,
        as_start_time -> Nullable<Timestamptz>,
        as_summary -> Nullable<Text>,
        as_summary_map -> Nullable<Jsonb>,
        as_tag -> Nullable<Jsonb>,
        as_to -> Nullable<Jsonb>,
        as_type -> ObjectType,
        as_updated -> Nullable<Timestamptz>,
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
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        uuid -> Varchar,
        profile_id -> Int4,
        olm_id -> Int4,
        key_data -> Varchar,
        distributed -> Bool,
    }
}

diesel::table! {
    olm_sessions (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        uuid -> Varchar,
        session_data -> Varchar,
        session_hash -> Varchar,
        encrypted_session_id -> Int4,
    }
}

diesel::table! {
    processing_queue (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        ap_id -> Varchar,
        ap_to -> Jsonb,
        cc -> Nullable<Jsonb>,
        attributed_to -> Varchar,
        kind -> Varchar,
        ap_object -> Jsonb,
        processed -> Bool,
        profile_id -> Int4,
    }
}

diesel::table! {
    remote_encrypted_sessions (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        profile_id -> Int4,
        actor -> Varchar,
        kind -> Varchar,
        ap_id -> Varchar,
        ap_to -> Varchar,
        attributed_to -> Varchar,
        instrument -> Jsonb,
        reference -> Nullable<Varchar>,
    }
}

diesel::table! {
    unprocessable (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        raw -> Jsonb,
    }
}

diesel::table! {
    vault (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        uuid -> Varchar,
        profile_id -> Int4,
        encrypted_data -> Varchar,
        remote_actor -> Varchar,
        outbound -> Bool,
    }
}

diesel::joinable!(olm_sessions -> encrypted_sessions (encrypted_session_id));

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
    notifications,
    objects,
    olm_one_time_keys,
    olm_sessions,
    processing_queue,
    remote_encrypted_sessions,
    unprocessable,
    vault,
);
