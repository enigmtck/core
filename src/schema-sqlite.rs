// @generated automatically by Diesel CLI.

diesel::table! {
    activities (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        profile_id -> Nullable<Integer>,
        kind -> Text,
        uuid -> Text,
        actor -> Text,
        ap_to -> Nullable<Text>,
        cc -> Nullable<Text>,
        target_note_id -> Nullable<Integer>,
        target_remote_note_id -> Nullable<Integer>,
        target_profile_id -> Nullable<Integer>,
        target_activity_id -> Nullable<Integer>,
        target_ap_id -> Nullable<Text>,
        target_remote_actor_id -> Nullable<Integer>,
        revoked -> Bool,
        ap_id -> Nullable<Text>,
        target_remote_question_id -> Nullable<Integer>,
    }
}

diesel::table! {
    activities_cc (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        activity_id -> Integer,
        ap_id -> Text,
    }
}

diesel::table! {
    activities_to (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        activity_id -> Integer,
        ap_id -> Text,
    }
}

diesel::table! {
    announces (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        profile_id -> Nullable<Integer>,
        uuid -> Text,
        actor -> Text,
        ap_to -> Text,
        cc -> Nullable<Text>,
        object_ap_id -> Text,
    }
}

diesel::table! {
    cache (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        uuid -> Text,
        url -> Text,
        media_type -> Nullable<Text>,
        height -> Nullable<Integer>,
        width -> Nullable<Integer>,
        blurhash -> Nullable<Text>,
    }
}

diesel::table! {
    encrypted_sessions (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        profile_id -> Integer,
        ap_to -> Text,
        attributed_to -> Text,
        instrument -> Text,
        reference -> Nullable<Text>,
        uuid -> Text,
    }
}

diesel::table! {
    followers (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        profile_id -> Integer,
        ap_id -> Text,
        actor -> Text,
        followed_ap_id -> Text,
        uuid -> Text,
    }
}

diesel::table! {
    follows (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        uuid -> Text,
        profile_id -> Nullable<Integer>,
        ap_object -> Text,
        actor -> Text,
    }
}

diesel::table! {
    hashtag_trend (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        period -> Integer,
        hashtag -> Text,
        update_count -> Integer,
    }
}

diesel::table! {
    instances (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        domain_name -> Text,
        json -> Nullable<Text>,
        blocked -> Bool,
        last_message_at -> Timestamp,
    }
}

diesel::table! {
    leaders (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        profile_id -> Integer,
        actor -> Text,
        leader_ap_id -> Text,
        uuid -> Text,
        accept_ap_id -> Nullable<Text>,
        accepted -> Nullable<Bool>,
        follow_ap_id -> Nullable<Text>,
    }
}

diesel::table! {
    likes (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        uuid -> Text,
        profile_id -> Nullable<Integer>,
        ap_to -> Text,
        actor -> Text,
        object_ap_id -> Text,
    }
}

diesel::table! {
    note_hashtags (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        hashtag -> Text,
        note_id -> Integer,
    }
}

diesel::table! {
    notes (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        uuid -> Text,
        profile_id -> Integer,
        kind -> Text,
        ap_to -> Text,
        cc -> Nullable<Text>,
        tag -> Nullable<Text>,
        attributed_to -> Text,
        in_reply_to -> Nullable<Text>,
        content -> Text,
        conversation -> Nullable<Text>,
        attachment -> Nullable<Text>,
        instrument -> Nullable<Text>,
        ap_id -> Nullable<Text>,
    }
}

diesel::table! {
    notifications (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        uuid -> Text,
        kind -> Text,
        profile_id -> Integer,
        activity_id -> Integer,
    }
}

diesel::table! {
    olm_one_time_keys (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        uuid -> Text,
        profile_id -> Integer,
        olm_id -> Integer,
        key_data -> Text,
        distributed -> Bool,
    }
}

diesel::table! {
    olm_sessions (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        uuid -> Text,
        session_data -> Text,
        session_hash -> Text,
        encrypted_session_id -> Integer,
    }
}

diesel::table! {
    processing_queue (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        ap_id -> Text,
        ap_to -> Text,
        cc -> Nullable<Text>,
        attributed_to -> Text,
        kind -> Text,
        ap_object -> Text,
        processed -> Bool,
        profile_id -> Integer,
    }
}

diesel::table! {
    profile_hashtags (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        hashtag -> Text,
        profile_id -> Integer,
    }
}

diesel::table! {
    profiles (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        uuid -> Text,
        username -> Text,
        display_name -> Text,
        summary -> Nullable<Text>,
        public_key -> Text,
        private_key -> Text,
        password -> Nullable<Text>,
        client_public_key -> Nullable<Text>,
        avatar_filename -> Nullable<Text>,
        banner_filename -> Nullable<Text>,
        salt -> Nullable<Text>,
        client_private_key -> Nullable<Text>,
        olm_pickled_account -> Nullable<Text>,
        olm_pickled_account_hash -> Nullable<Text>,
        olm_identity_key -> Nullable<Text>,
        summary_markdown -> Nullable<Text>,
    }
}

diesel::table! {
    remote_actor_hashtags (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        hashtag -> Text,
        remote_actor_id -> Integer,
    }
}

diesel::table! {
    remote_actors (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        context -> Text,
        kind -> Text,
        ap_id -> Text,
        name -> Text,
        preferred_username -> Nullable<Text>,
        summary -> Nullable<Text>,
        inbox -> Text,
        outbox -> Text,
        followers -> Nullable<Text>,
        following -> Nullable<Text>,
        liked -> Nullable<Text>,
        public_key -> Text,
        featured -> Nullable<Text>,
        featured_tags -> Nullable<Text>,
        url -> Nullable<Text>,
        manually_approves_followers -> Nullable<Bool>,
        published -> Nullable<Text>,
        tag -> Nullable<Text>,
        attachment -> Nullable<Text>,
        endpoints -> Nullable<Text>,
        icon -> Nullable<Text>,
        image -> Nullable<Text>,
        also_known_as -> Nullable<Text>,
        discoverable -> Nullable<Bool>,
        capabilities -> Nullable<Text>,
        checked_at -> Timestamp,
        webfinger -> Nullable<Text>,
    }
}

diesel::table! {
    remote_encrypted_sessions (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        profile_id -> Integer,
        actor -> Text,
        kind -> Text,
        ap_id -> Text,
        ap_to -> Text,
        attributed_to -> Text,
        instrument -> Text,
        reference -> Nullable<Text>,
    }
}

diesel::table! {
    remote_note_hashtags (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        hashtag -> Text,
        remote_note_id -> Integer,
    }
}

diesel::table! {
    remote_notes (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        kind -> Text,
        ap_id -> Text,
        published -> Nullable<Text>,
        url -> Nullable<Text>,
        ap_to -> Nullable<Text>,
        cc -> Nullable<Text>,
        tag -> Nullable<Text>,
        attributed_to -> Text,
        content -> Text,
        attachment -> Nullable<Text>,
        replies -> Nullable<Text>,
        in_reply_to -> Nullable<Text>,
        signature -> Nullable<Text>,
        summary -> Nullable<Text>,
        ap_sensitive -> Nullable<Bool>,
        atom_uri -> Nullable<Text>,
        in_reply_to_atom_uri -> Nullable<Text>,
        conversation -> Nullable<Text>,
        content_map -> Nullable<Text>,
    }
}

diesel::table! {
    remote_questions (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        kind -> Text,
        ap_id -> Text,
        ap_to -> Nullable<Text>,
        cc -> Nullable<Text>,
        end_time -> Nullable<Timestamp>,
        published -> Nullable<Timestamp>,
        one_of -> Nullable<Text>,
        any_of -> Nullable<Text>,
        content -> Nullable<Text>,
        content_map -> Nullable<Text>,
        summary -> Nullable<Text>,
        voters_count -> Nullable<Integer>,
        url -> Nullable<Text>,
        conversation -> Nullable<Text>,
        tag -> Nullable<Text>,
        attachment -> Nullable<Text>,
        ap_sensitive -> Nullable<Bool>,
        in_reply_to -> Nullable<Text>,
        attributed_to -> Text,
    }
}

diesel::table! {
    timeline (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        tag -> Nullable<Text>,
        attributed_to -> Text,
        ap_id -> Text,
        kind -> Text,
        url -> Nullable<Text>,
        published -> Nullable<Text>,
        replies -> Nullable<Text>,
        in_reply_to -> Nullable<Text>,
        content -> Nullable<Text>,
        ap_public -> Bool,
        summary -> Nullable<Text>,
        ap_sensitive -> Nullable<Bool>,
        atom_uri -> Nullable<Text>,
        in_reply_to_atom_uri -> Nullable<Text>,
        conversation -> Nullable<Text>,
        content_map -> Nullable<Text>,
        attachment -> Nullable<Text>,
        ap_object -> Nullable<Text>,
        metadata -> Nullable<Text>,
        end_time -> Nullable<Timestamp>,
        one_of -> Nullable<Text>,
        any_of -> Nullable<Text>,
        voters_count -> Nullable<Integer>,
    }
}

diesel::table! {
    timeline_cc (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        timeline_id -> Integer,
        ap_id -> Text,
    }
}

diesel::table! {
    timeline_hashtags (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        hashtag -> Text,
        timeline_id -> Integer,
    }
}

diesel::table! {
    timeline_to (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        timeline_id -> Integer,
        ap_id -> Text,
    }
}

diesel::table! {
    vault (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        uuid -> Text,
        profile_id -> Integer,
        encrypted_data -> Text,
        remote_actor -> Text,
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
diesel::joinable!(timeline_cc -> timeline (timeline_id));
diesel::joinable!(timeline_hashtags -> timeline (timeline_id));
diesel::joinable!(timeline_to -> timeline (timeline_id));
diesel::joinable!(vault -> profiles (profile_id));

diesel::allow_tables_to_appear_in_same_query!(
    activities,
    activities_cc,
    activities_to,
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
    timeline,
    timeline_cc,
    timeline_hashtags,
    timeline_to,
    vault,
);
