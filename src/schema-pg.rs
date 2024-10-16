// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
    #[diesel(postgres_type(name = "activity_type"))]
    pub struct ActivityType;

    #[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
    #[diesel(postgres_type(name = "actor_type"))]
    pub struct ActorType;

    #[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
    #[diesel(postgres_type(name = "note_type"))]
    pub struct NoteType;

    #[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
    #[diesel(postgres_type(name = "notification_type"))]
    pub struct NotificationType;

    #[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
    #[diesel(postgres_type(name = "object_type"))]
    pub struct ObjectType;

    #[derive(diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
    #[diesel(postgres_type(name = "question_type"))]
    pub struct QuestionType;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ActivityType;

    activities (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        profile_id -> Nullable<Int4>,
        kind -> ActivityType,
        uuid -> Varchar,
        actor -> Varchar,
        ap_to -> Nullable<Jsonb>,
        cc -> Nullable<Jsonb>,
        target_note_id -> Nullable<Int4>,
        target_remote_note_id -> Nullable<Int4>,
        target_profile_id -> Nullable<Int4>,
        target_activity_id -> Nullable<Int4>,
        target_ap_id -> Nullable<Varchar>,
        target_remote_actor_id -> Nullable<Int4>,
        revoked -> Bool,
        ap_id -> Nullable<Varchar>,
        target_remote_question_id -> Nullable<Int4>,
        reply -> Bool,
        raw -> Nullable<Jsonb>,
        target_object_id -> Nullable<Int4>,
        actor_id -> Nullable<Int4>,
        target_actor_id -> Nullable<Int4>,
    }
}

diesel::table! {
    activities_cc (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        activity_id -> Int4,
        ap_id -> Varchar,
    }
}

diesel::table! {
    activities_to (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        activity_id -> Int4,
        ap_id -> Varchar,
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
    announces (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        profile_id -> Nullable<Int4>,
        uuid -> Varchar,
        actor -> Varchar,
        ap_to -> Jsonb,
        cc -> Nullable<Jsonb>,
        object_ap_id -> Varchar,
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
        profile_id -> Int4,
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
        profile_id -> Int4,
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
    likes (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        uuid -> Varchar,
        profile_id -> Nullable<Int4>,
        ap_to -> Varchar,
        actor -> Varchar,
        object_ap_id -> Varchar,
    }
}

diesel::table! {
    note_hashtags (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        hashtag -> Varchar,
        note_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::NoteType;

    notes (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        uuid -> Varchar,
        profile_id -> Int4,
        kind -> NoteType,
        ap_to -> Jsonb,
        cc -> Nullable<Jsonb>,
        tag -> Nullable<Jsonb>,
        attributed_to -> Varchar,
        in_reply_to -> Nullable<Varchar>,
        content -> Varchar,
        conversation -> Nullable<Varchar>,
        attachment -> Nullable<Jsonb>,
        instrument -> Nullable<Jsonb>,
        ap_id -> Nullable<Varchar>,
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
    profile_hashtags (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        hashtag -> Varchar,
        profile_id -> Int4,
    }
}

diesel::table! {
    profiles (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        uuid -> Varchar,
        username -> Varchar,
        display_name -> Varchar,
        summary -> Nullable<Varchar>,
        public_key -> Varchar,
        private_key -> Varchar,
        password -> Nullable<Varchar>,
        client_public_key -> Nullable<Varchar>,
        avatar_filename -> Nullable<Varchar>,
        banner_filename -> Nullable<Varchar>,
        salt -> Nullable<Varchar>,
        client_private_key -> Nullable<Varchar>,
        olm_pickled_account -> Nullable<Varchar>,
        olm_pickled_account_hash -> Nullable<Varchar>,
        olm_identity_key -> Nullable<Varchar>,
        summary_markdown -> Nullable<Varchar>,
    }
}

diesel::table! {
    remote_actor_hashtags (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        hashtag -> Varchar,
        remote_actor_id -> Int4,
    }
}

diesel::table! {
    remote_actors (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        context -> Jsonb,
        kind -> Varchar,
        ap_id -> Varchar,
        name -> Varchar,
        preferred_username -> Nullable<Varchar>,
        summary -> Nullable<Varchar>,
        inbox -> Varchar,
        outbox -> Varchar,
        followers -> Nullable<Varchar>,
        following -> Nullable<Varchar>,
        liked -> Nullable<Varchar>,
        public_key -> Nullable<Jsonb>,
        featured -> Nullable<Varchar>,
        featured_tags -> Nullable<Varchar>,
        url -> Nullable<Varchar>,
        manually_approves_followers -> Nullable<Bool>,
        published -> Nullable<Varchar>,
        tag -> Nullable<Jsonb>,
        attachment -> Nullable<Jsonb>,
        endpoints -> Nullable<Jsonb>,
        icon -> Nullable<Jsonb>,
        image -> Nullable<Jsonb>,
        also_known_as -> Nullable<Jsonb>,
        discoverable -> Nullable<Bool>,
        capabilities -> Nullable<Jsonb>,
        checked_at -> Timestamptz,
        webfinger -> Nullable<Varchar>,
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
    remote_note_hashtags (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        hashtag -> Varchar,
        remote_note_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::NoteType;

    remote_notes (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        kind -> NoteType,
        ap_id -> Varchar,
        published -> Nullable<Varchar>,
        url -> Nullable<Varchar>,
        ap_to -> Nullable<Jsonb>,
        cc -> Nullable<Jsonb>,
        tag -> Nullable<Jsonb>,
        attributed_to -> Varchar,
        content -> Varchar,
        attachment -> Nullable<Jsonb>,
        replies -> Nullable<Jsonb>,
        in_reply_to -> Nullable<Varchar>,
        signature -> Nullable<Jsonb>,
        summary -> Nullable<Varchar>,
        ap_sensitive -> Nullable<Bool>,
        atom_uri -> Nullable<Varchar>,
        in_reply_to_atom_uri -> Nullable<Varchar>,
        conversation -> Nullable<Varchar>,
        content_map -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::QuestionType;

    remote_questions (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        kind -> QuestionType,
        ap_id -> Varchar,
        ap_to -> Nullable<Jsonb>,
        cc -> Nullable<Jsonb>,
        end_time -> Nullable<Timestamptz>,
        published -> Nullable<Timestamptz>,
        one_of -> Nullable<Jsonb>,
        any_of -> Nullable<Jsonb>,
        content -> Nullable<Varchar>,
        content_map -> Nullable<Jsonb>,
        summary -> Nullable<Varchar>,
        voters_count -> Nullable<Int4>,
        url -> Nullable<Text>,
        conversation -> Nullable<Text>,
        tag -> Nullable<Jsonb>,
        attachment -> Nullable<Jsonb>,
        ap_sensitive -> Nullable<Bool>,
        in_reply_to -> Nullable<Text>,
        attributed_to -> Varchar,
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

diesel::joinable!(activities_cc -> activities (activity_id));
diesel::joinable!(activities_to -> activities (activity_id));
diesel::joinable!(encrypted_sessions -> profiles (profile_id));
diesel::joinable!(followers -> profiles (profile_id));
diesel::joinable!(leaders -> profiles (profile_id));
diesel::joinable!(note_hashtags -> remote_notes (note_id));
diesel::joinable!(notes -> profiles (profile_id));
diesel::joinable!(olm_one_time_keys -> profiles (profile_id));
diesel::joinable!(olm_sessions -> encrypted_sessions (encrypted_session_id));
diesel::joinable!(profile_hashtags -> profiles (profile_id));
diesel::joinable!(remote_actor_hashtags -> remote_actors (remote_actor_id));
diesel::joinable!(remote_encrypted_sessions -> profiles (profile_id));
diesel::joinable!(remote_note_hashtags -> remote_notes (remote_note_id));
diesel::joinable!(vault -> profiles (profile_id));

diesel::allow_tables_to_appear_in_same_query!(
    activities,
    activities_cc,
    activities_to,
    actors,
    announces,
    cache,
    encrypted_sessions,
    followers,
    follows,
    hashtag_trend,
    instances,
    leaders,
    likes,
    note_hashtags,
    notes,
    notifications,
    objects,
    olm_one_time_keys,
    olm_sessions,
    processing_queue,
    profile_hashtags,
    profiles,
    remote_actor_hashtags,
    remote_actors,
    remote_encrypted_sessions,
    remote_note_hashtags,
    remote_notes,
    remote_questions,
    vault,
);
