// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "activity_type"))]
    pub struct ActivityType;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "note_type"))]
    pub struct NoteType;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ActivityType;

    activities (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        profile_id -> Int4,
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
        avatar_filename -> Varchar,
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
    remote_activities (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        context -> Nullable<Jsonb>,
        kind -> Varchar,
        ap_id -> Varchar,
        ap_to -> Nullable<Jsonb>,
        cc -> Nullable<Jsonb>,
        actor -> Varchar,
        published -> Nullable<Varchar>,
        ap_object -> Nullable<Jsonb>,
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
    remote_announces (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        context -> Nullable<Varchar>,
        kind -> Varchar,
        ap_id -> Varchar,
        actor -> Varchar,
        ap_to -> Nullable<Jsonb>,
        cc -> Nullable<Jsonb>,
        published -> Varchar,
        ap_object -> Jsonb,
        timeline_id -> Nullable<Int4>,
        revoked -> Bool,
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
    remote_likes (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        ap_id -> Varchar,
        actor -> Varchar,
        object_id -> Varchar,
    }
}

diesel::table! {
    remote_notes (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        kind -> Varchar,
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
    }
}

diesel::table! {
    timeline (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        tag -> Nullable<Jsonb>,
        attributed_to -> Varchar,
        ap_id -> Varchar,
        kind -> Varchar,
        url -> Nullable<Varchar>,
        published -> Nullable<Varchar>,
        replies -> Nullable<Jsonb>,
        in_reply_to -> Nullable<Varchar>,
        content -> Nullable<Varchar>,
        ap_public -> Bool,
        summary -> Nullable<Varchar>,
        ap_sensitive -> Nullable<Bool>,
        atom_uri -> Nullable<Varchar>,
        in_reply_to_atom_uri -> Nullable<Varchar>,
        conversation -> Nullable<Varchar>,
        content_map -> Nullable<Jsonb>,
        attachment -> Nullable<Jsonb>,
        ap_object -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    timeline_cc (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        timeline_id -> Int4,
        ap_id -> Varchar,
    }
}

diesel::table! {
    timeline_to (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        timeline_id -> Int4,
        ap_id -> Varchar,
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

diesel::joinable!(activities -> profiles (profile_id));
diesel::joinable!(encrypted_sessions -> profiles (profile_id));
diesel::joinable!(followers -> profiles (profile_id));
diesel::joinable!(leaders -> profiles (profile_id));
diesel::joinable!(notes -> profiles (profile_id));
diesel::joinable!(olm_one_time_keys -> profiles (profile_id));
diesel::joinable!(olm_sessions -> encrypted_sessions (encrypted_session_id));
diesel::joinable!(remote_announces -> timeline (timeline_id));
diesel::joinable!(remote_encrypted_sessions -> profiles (profile_id));
diesel::joinable!(timeline_cc -> timeline (timeline_id));
diesel::joinable!(timeline_to -> timeline (timeline_id));
diesel::joinable!(vault -> profiles (profile_id));

diesel::allow_tables_to_appear_in_same_query!(
    activities,
    announces,
    encrypted_sessions,
    followers,
    follows,
    leaders,
    likes,
    notes,
    olm_one_time_keys,
    olm_sessions,
    processing_queue,
    profiles,
    remote_activities,
    remote_actors,
    remote_announces,
    remote_encrypted_sessions,
    remote_likes,
    remote_notes,
    timeline,
    timeline_cc,
    timeline_to,
    vault,
);
