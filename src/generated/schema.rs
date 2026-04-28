// @generated automatically by Diesel CLI.

diesel::table! {
    identities (id) {
        id -> Integer,
        created_at -> Timestamp,
        key -> Binary,
        last_tcp_port -> Nullable<Integer>,
        last_quic_port -> Nullable<Integer>,
        self_nickname -> Nullable<Text>,
    }
}

diesel::table! {
    message_receipts (id) {
        id -> Integer,
        msg_id -> Text,
        peer_id -> Text,
        kind -> Integer,
        confirmed_at -> Double,
        created_at -> Timestamp,
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
        msg_id -> Nullable<Text>,
        sent_at -> Nullable<Double>,
    }
}

diesel::table! {
    peer_sessions (id) {
        id -> Integer,
        concurrent_peers -> Integer,
        recorded_at -> Timestamp,
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
        peer_local_nickname -> Nullable<Text>,
        self_nickname_for_peer -> Nullable<Text>,
        received_nickname -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    identities,
    message_receipts,
    messages,
    peer_sessions,
    peers,
);
