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
        is_direct -> Integer,
        target_peer -> Nullable<Text>,
    }
}

diesel::table! {
    peers (id) {
        id -> Integer,
        created_at -> Timestamp,
        peer_id -> Text,
        addresses -> Text,
        first_seen -> Timestamp,
        last_seen -> Timestamp,
    }
}

diesel::joinable!(messages -> identities (id));
diesel::joinable!(peers -> identities (id));

diesel::allow_tables_to_appear_in_same_query!(identities, messages, peers,);
