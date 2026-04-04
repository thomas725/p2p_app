// @generated automatically by Diesel CLI.

diesel::table! {
    identities (id) {
        id -> Integer,
        created_at -> Timestamp,
        key -> Binary,
    }
}

diesel::table! {
    messages (id) {
        id -> Integer,
        created_at -> Timestamp,
        content -> Text,
        peer_id -> Nullable<Text>,
        topic -> Text,
        sent -> Integer,
    }
}

diesel::joinable!(messages -> identities (id));

diesel::allow_tables_to_appear_in_same_query!(identities, messages,);
