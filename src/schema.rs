// @generated automatically by Diesel CLI.

diesel::table! {
    note_subjects (id) {
        id -> Int4,
        note_id -> Int4,
        profile_id -> Int4,
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

diesel::joinable!(note_subjects -> notes (note_id));
diesel::joinable!(note_subjects -> profiles (profile_id));
diesel::joinable!(notes -> profiles (profile_id));

diesel::allow_tables_to_appear_in_same_query!(
    note_subjects,
    notes,
    profiles,
    remote_actors,
);
