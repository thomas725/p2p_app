// @generated automatically by Diesel CLI.

#![allow(missing_docs)]

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
        sender_nickname -> Nullable<Text>,
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
        generated_nickname -> Nullable<Text>,
        broadcasts_sent -> Integer,
    }
}

// Hand-maintained (not produced by Diesel CLI): keeps a timestamped record of
// every name a peer has used. Lives here so that `flutter_rust_bridge` can
// ignore this whole module (its `table!` macros expand to unit structs that
// the FRB parser rejects).
diesel::table! {
    peer_name_history (id) {
        id -> Integer,
        peer_id -> Text,
        name -> Text,
        name_kind -> Text,
        set_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    identities,
    message_receipts,
    messages,
    peer_name_history,
    peer_sessions,
    peers,
);
