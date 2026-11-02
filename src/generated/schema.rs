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
        account_key -> Nullable<Binary>,
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
        attachment_json -> Nullable<Text>,
        encrypted -> Integer,
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

// Hand-maintained: records, per broadcast *we* sent, which peers we transmitted
// it to and (once they acknowledge) when they confirmed receipt. Drives the
// "Broadcast" (broadcasts sent-to-peer) counter in both frontends' peer tables,
// replacing the dormant `peers.broadcasts_sent` aggregate. Lives here (not the
// migrations-only model) so diesel can query it and FRB ignores this module.
diesel::table! {
    broadcast_recipients (id) {
        id -> Integer,
        msg_id -> Text,
        peer_id -> Text,
        sent_at -> Double,
        confirmed_at -> Nullable<Double>,
    }
}

// Hand-maintained: group-chat tables (`groups`, `group_members`,
// `group_messages`) for public/private group chats transported over per-group
// gossipsub topics. `group_id` is a stable hash of the canonical group name
// (public groups identify by name); `is_private` reserves the private-group
// switch. Lives here so diesel can query it and FRB ignores this module.
diesel::table! {
    groups (id) {
        id -> Integer,
        group_id -> Text,
        display_name -> Text,
        is_private -> Integer,
        created_at -> Timestamp,
    }
}

diesel::table! {
    group_members (id) {
        id -> Integer,
        group_id -> Text,
        peer_id -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    group_messages (id) {
        id -> Integer,
        created_at -> Timestamp,
        group_id -> Text,
        content -> Text,
        peer_id -> Nullable<Text>,
        sent -> Integer,
        msg_id -> Nullable<Text>,
        sent_at -> Nullable<Double>,
        sender_nickname -> Nullable<Text>,
        attachment_json -> Nullable<Text>,
    }
}

// Hand-maintained (not produced by Diesel CLI): identity lane lanes ("Identity
// and contacts" backlog §11; design doc 2026-09-21). `identities` stays the
// per-install libp2p keypair; these three tables lift the app layer *above* it:
//
//   users             stable account (root of trust). `account_pubkey` is the
//                     BLOB account key that signs every device-binding claim.
//                     `self_username`/`self_nickname` are the display lane.
//   device_bindings   signed device→user claims. A row only exists after the
//                     claim's signature verifies against *that user's* account
//                     pubkey (hostile-device invariant — a device can never
//                     mint a binding into a foreign user's device-set).
//   contacts          user→user adjacency (contact list; device-agnostic,
//                     unlike `peers` which is install/device-scoped).
// lives here so diesel can query it and FRB ignores this module.
diesel::table! {
    users (user_id) {
        user_id -> Text,
        account_pubkey -> Binary,
        created_at -> Timestamp,
        self_username -> Nullable<Text>,
        self_nickname -> Nullable<Text>,
    }
}

diesel::table! {
    device_bindings (claim_id) {
        claim_id -> Text,
        user_id -> Text,
        peer_id -> Text,
        device_pubkey -> Binary,
        claim -> Binary,
        bound_at -> Timestamp,
        revoked_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    contacts (owner_user_id, contact_user_id) {
        owner_user_id -> Text,
        contact_user_id -> Text,
        nickname_override -> Nullable<Text>,
        added_at -> Timestamp,
        last_seen -> Nullable<Timestamp>,
    }
}

// Hand-maintained: content-addressed attachment store (files lane §12). The
// bytes live on disk under `files_dir()` keyed by `file_id` = SHA-256 hex of
// the content; this table records delivery-facing metadata plus the absolute
// stored path so `serve_file_*` and the frontends' open/save affordances have
// one lookup. Lives here so diesel can query it and FRB ignores this module.
diesel::table! {
    files (file_id) {
        file_id -> Text,
        name -> Text,
        size -> BigInt,
        mime -> Nullable<Text>,
        direction -> Integer,
        path -> Text,
        created_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    broadcast_recipients,
    contacts,
    device_bindings,
    files,
    group_members,
    group_messages,
    groups,
    identities,
    message_receipts,
    messages,
    peer_name_history,
    peer_sessions,
    peers,
    users,
);
