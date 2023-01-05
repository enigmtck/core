// @generated automatically by Diesel CLI.

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
    notes (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        uuid -> Varchar,
        profile_id -> Int4,
        content -> Varchar,
        ap_to -> Jsonb,
        ap_tag -> Nullable<Jsonb>,
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
        keystore -> Nullable<Jsonb>,
        client_public_key -> Nullable<Varchar>,
    }
}

diesel::table! {
    remote_activities (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        profile_id -> Int4,
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
        followers -> Varchar,
        following -> Varchar,
        liked -> Nullable<Varchar>,
        public_key -> Nullable<Jsonb>,
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
    remote_notes (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        profile_id -> Int4,
        ap_id -> Varchar,
        published -> Nullable<Varchar>,
        url -> Nullable<Varchar>,
        attributed_to -> Nullable<Varchar>,
        ap_to -> Nullable<Jsonb>,
        cc -> Nullable<Jsonb>,
        content -> Varchar,
        attachment -> Nullable<Jsonb>,
        tag -> Nullable<Jsonb>,
        replies -> Nullable<Jsonb>,
        signature -> Nullable<Jsonb>,
    }
}

diesel::joinable!(encrypted_sessions -> profiles (profile_id));
diesel::joinable!(followers -> profiles (profile_id));
diesel::joinable!(leaders -> profiles (profile_id));
diesel::joinable!(remote_encrypted_sessions -> profiles (profile_id));

diesel::allow_tables_to_appear_in_same_query!(
    encrypted_sessions,
    followers,
    leaders,
    notes,
    profiles,
    remote_activities,
    remote_actors,
    remote_encrypted_sessions,
    remote_notes,
);
