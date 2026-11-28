#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction,
    clippy::used_underscore_binding,
    clippy::significant_drop_tightening
)]

use super::*;
use crate::generated::models_queryable::{GroupMessage, Message};
use crate::types::MessageEvent;
use chrono::NaiveDateTime;
use serial_test::serial;

fn dt(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
}

// ── event_to_json ──────────────────────────────────────────────────

#[test]
fn test_event_to_json_broadcast() {
    let attachment = crate::files::MessageAttachment {
        file_id: "abc123".into(),
        name: "notes.txt".into(),
        size: 2048,
        mime: Some("text/plain".into()),
    };
    let ev = SwarmEvent::BroadcastMessage(MessageEvent {
        content: "hello".into(),
        peer_id: "p1".into(),
        latency: Some("5ms".into()),
        nickname: Some("Alice".into()),
        msg_id: Some("m1".into()),
        attachment: Some(attachment),
        reply_to: None,
        encrypted: false,
    });
    let j = event_to_json(ev);
    assert_eq!(j.event_type, "broadcast");
    assert_eq!(j.peer_id.as_deref(), Some("p1"));
    assert_eq!(j.content.as_deref(), Some("hello"));
    assert_eq!(j.latency.as_deref(), Some("5ms"));
    assert_eq!(j.nickname.as_deref(), Some("Alice"));
    assert_eq!(j.msg_id.as_deref(), Some("m1"));
    assert!(j.address.is_none());
    let forwarded = j.attachment.expect("live attachment reaches the event");
    assert_eq!(forwarded.file_id, "abc123");
    assert_eq!(forwarded.size_label, "2 KiB");
    assert_eq!(forwarded.chip, "[file] notes.txt (2 KiB)");
    assert_eq!(forwarded.mime.as_deref(), Some("text/plain"));
}

#[test]
fn test_event_to_json_direct_message() {
    let ev = SwarmEvent::DirectMessage(MessageEvent {
        content: "secret".into(),
        peer_id: "p2".into(),
        latency: None,
        nickname: None,
        msg_id: None,
        attachment: None,
        reply_to: None,
        encrypted: false,
    });
    let j = event_to_json(ev);
    assert_eq!(j.event_type, "dm");
    assert_eq!(j.peer_id.as_deref(), Some("p2"));
    assert_eq!(j.content.as_deref(), Some("secret"));
    assert!(j.latency.is_none());
    assert!(j.nickname.is_none());
}

#[test]
fn test_event_to_json_peer_connected() {
    let j = event_to_json(SwarmEvent::PeerConnected("p1".into()));
    assert_eq!(j.event_type, "peer_connected");
    assert_eq!(j.peer_id.as_deref(), Some("p1"));
    assert!(j.content.is_none());
}

#[test]
#[serial(db)]
fn test_event_to_json_peer_disconnected() {
    // The disconnect event carries the canonical Rust display label, so
    // isolate the DB and clear the global name cache for a deterministic
    // assertion (resolution can persist a generated name).
    let _guard = crate::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::tempdir().expect("tempdir");
    let db_url = dir
        .path()
        .join("disconnected.sqlite")
        .to_string_lossy()
        .into_owned();
    crate::reset_db_url();
    crate::db::set_db_url(&db_url);
    crate::init_database().expect("init db");
    crate::nickname::clear_display_names();
    crate::nickname::set_peer_local_nickname("peer-disconnect-label", "Alice")
        .expect("set nickname");

    let j = event_to_json(SwarmEvent::PeerDisconnected("peer-disconnect-label".into()));
    assert_eq!(j.event_type, "peer_disconnected");
    assert_eq!(j.peer_id.as_deref(), Some("peer-disconnect-label"));
    assert_eq!(j.content.as_deref(), Some("Alice (bel)"));
}

#[test]
fn test_event_to_json_listen_addr() {
    let j = event_to_json(SwarmEvent::ListenAddrEstablished(
        "/ip4/0.0.0.0/tcp/4000".into(),
    ));
    assert_eq!(j.event_type, "listen_addr");
    assert_eq!(j.address.as_deref(), Some("/ip4/0.0.0.0/tcp/4000"));
}

#[test]
fn test_event_to_json_receipt() {
    let j = event_to_json(SwarmEvent::Receipt {
        peer_id: "p1".into(),
        ack_for: "msg-42".into(),
        received_at: Some(1000.0),
    });
    assert_eq!(j.event_type, "receipt");
    assert_eq!(j.peer_id.as_deref(), Some("p1"));
    assert_eq!(j.msg_id.as_deref(), Some("msg-42"));
}

#[test]
fn test_event_to_json_typing() {
    for (is_typing, expected) in [(true, "start"), (false, "stop")] {
        let j = event_to_json(SwarmEvent::Typing {
            peer_id: "p9".into(),
            is_typing,
        });
        assert_eq!(j.event_type, "typing");
        assert_eq!(j.peer_id.as_deref(), Some("p9"));
        assert_eq!(j.content.as_deref(), Some(expected));
    }
}

#[cfg(feature = "mdns")]
#[test]
fn test_event_to_json_peer_discovered() {
    let addr: libp2p::Multiaddr = "/ip4/10.0.0.1/tcp/9000".parse().unwrap();
    let j = event_to_json(SwarmEvent::PeerDiscovered {
        peer_id: "p5".into(),
        addresses: vec![addr],
    });
    assert_eq!(j.event_type, "peer_discovered");
    assert_eq!(j.peer_id.as_deref(), Some("p5"));
    assert!(j.address.is_some());
}

#[cfg(feature = "mdns")]
#[test]
fn test_event_to_json_peer_expired() {
    let j = event_to_json(SwarmEvent::PeerExpired {
        peer_id: "p6".into(),
    });
    assert_eq!(j.event_type, "peer_expired");
    assert_eq!(j.peer_id.as_deref(), Some("p6"));
}

#[test]
fn test_event_to_json_share_offered_uses_camel_case() {
    // Dart reads an offer's `content` with the camelCase keys generated by
    // FRB. The domain types keep snake_case for the wire/DB, so the event
    // boundary must convert explicitly (a silent default would lose every
    // value). See `shares_json.dart`.
    let j = event_to_json(SwarmEvent::ShareOffered {
        peer_id: "peer-7".into(),
        offer: crate::shares::ShareOffer {
            share_id: "share-42".into(),
            name: "Photos".into(),
            kind: crate::shares::ShareKind::Public,
            revision: 3,
            public_write: true,
            owner_user_id: Some("owner".into()),
        },
    });
    assert_eq!(j.event_type, "share_offered");
    assert_eq!(j.peer_id.as_deref(), Some("peer-7"));
    let content: serde_json::Value =
        serde_json::from_str(j.content.as_deref().expect("offer content")).expect("json");
    assert_eq!(content["shareId"], "share-42");
    assert_eq!(content["name"], "Photos");
    assert_eq!(content["kind"], "public");
    assert_eq!(content["revision"], 3);
    assert_eq!(content["publicWrite"], true);
    assert_eq!(content["ownerUserId"], "owner");
    assert!(content.get("share_id").is_none());
}

#[test]
fn test_event_to_json_call_invite() {
    let j = event_to_json(SwarmEvent::CallInvite {
        call_id: "c1".into(),
        peer_id: "p1".into(),
    });
    assert_eq!(j.event_type, "call_invite");
    assert_eq!(j.peer_id.as_deref(), Some("p1"));
    assert_eq!(j.content.as_deref(), Some("c1"));
}

#[test]
fn test_event_to_json_call_accepted() {
    let j = event_to_json(SwarmEvent::CallAccepted {
        call_id: "c1".into(),
        peer_id: "p1".into(),
    });
    assert_eq!(j.event_type, "call_accepted");
    assert_eq!(j.peer_id.as_deref(), Some("p1"));
    assert_eq!(j.content.as_deref(), Some("c1"));
}

#[test]
fn test_event_to_json_call_rejected() {
    let j = event_to_json(SwarmEvent::CallRejected {
        call_id: "c1".into(),
        peer_id: "p1".into(),
    });
    assert_eq!(j.event_type, "call_rejected");
    assert_eq!(j.peer_id.as_deref(), Some("p1"));
    assert_eq!(j.content.as_deref(), Some("c1"));
}

#[test]
fn test_event_to_json_call_ended() {
    let j = event_to_json(SwarmEvent::CallEnded {
        call_id: "c1".into(),
        peer_id: "p1".into(),
    });
    assert_eq!(j.event_type, "call_ended");
    assert_eq!(j.peer_id.as_deref(), Some("p1"));
    assert_eq!(j.content.as_deref(), Some("c1"));
}

// ── default_event ──────────────────────────────────────────────────

#[test]
fn test_default_event_fields() {
    let j = default_event();
    assert_eq!(j.event_type, "");
    assert!(j.peer_id.is_none());
    assert!(j.content.is_none());
    assert!(j.latency.is_none());
    assert!(j.nickname.is_none());
    assert!(j.msg_id.is_none());
    assert!(j.address.is_none());
}

// ── message_to_chat ────────────────────────────────────────────────

#[test]
fn test_message_to_chat_broadcast() {
    let msg = Message {
        id: 1,
        created_at: dt("2024-06-15 10:30:00"),
        content: "hello world".into(),
        peer_id: None,
        topic: "chat".into(),
        sent: 1,
        is_direct: 0,
        target_peer: None,
        msg_id: Some("msg-1".into()),
        sent_at: Some(1_718_455_800.0),
        sender_nickname: Some("Alice".into()),
        attachment_json: None,
        reply_to_json: None,
        encrypted: 0,
        deleted_at: None,
        edited_at: None,
    };
    let chat = message_to_chat(msg);
    assert_eq!(chat.id, 1);
    assert_eq!(chat.content, "hello world");
    assert!(chat.peer_id.is_none());
    assert!(chat.origin.is_broadcast);
    assert!(chat.origin.sent);
    assert_eq!(chat.msg_id.as_deref(), Some("msg-1"));
    assert!(chat.sent_at.is_some());
    assert_eq!(chat.sender_nickname.as_deref(), Some("Alice"));
}

#[test]
#[serial(db)]
fn test_message_to_chat_dm() {
    // Isolate DB state so the resolved display name is deterministic and
    // not contaminated by other tests running in parallel.
    let _guard = crate::db::shared_db_test_lock().lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir");
    let db_url = dir
        .path()
        .join("msg_to_chat.sqlite")
        .to_string_lossy()
        .into_owned();
    crate::reset_db_url();
    crate::db::set_db_url(&db_url);
    crate::init_database().expect("init db");

    let msg = Message {
        id: 2,
        created_at: dt("2024-06-15 11:00:00"),
        content: "secret msg".into(),
        peer_id: Some("peer-bob".into()),
        topic: "chat".into(),
        sent: 0,
        is_direct: 1,
        target_peer: Some("peer-bob".into()),
        msg_id: None,
        sent_at: None,
        sender_nickname: None,
        attachment_json: None,
        reply_to_json: None,
        encrypted: 0,
        deleted_at: None,
        edited_at: None,
    };
    let chat = message_to_chat(msg);
    assert_eq!(chat.id, 2);
    assert!(!chat.origin.is_broadcast);
    assert!(!chat.origin.sent);
    assert!(chat.sent_at.is_none());
    // No announced nickname: falls back to the resolved display name
    // (petname + short-id suffix), not a raw ID fragment.
    let expected = crate::get_peer_display_name("peer-bob").expect("resolved name");
    assert_eq!(chat.sender_nickname.as_deref(), Some(expected.as_str()));
    assert_eq!(chat.target_peer.as_deref(), Some("peer-bob"));
}

#[test]
#[serial(db)]
fn test_message_to_chat_announced_nickname_gets_suffix() {
    // Announced nickname + peer id => "Nick (short-id)" consistently.
    let msg = Message {
        id: 10,
        created_at: dt("2024-02-02 12:00:00"),
        content: "hello".into(),
        peer_id: Some("peer-bob".into()),
        topic: "chat".into(),
        sent: 0,
        is_direct: 0,
        target_peer: None,
        msg_id: None,
        sent_at: None,
        sender_nickname: Some("Bob".into()),
        attachment_json: None,
        reply_to_json: None,
        encrypted: 0,
        deleted_at: None,
        edited_at: None,
    };
    let chat = message_to_chat(msg);
    let suffix = crate::fmt::peer_id_suffix("peer-bob");
    assert_eq!(
        chat.sender_nickname.as_deref(),
        Some(format!("Bob ({suffix})").as_str())
    );
    assert!(chat.origin.is_broadcast);
}

#[test]
#[serial(db)]
fn test_message_to_chat_own_message_uses_nickname() {
    // No peer id (own message) => the announced nickname is used verbatim.
    let msg = Message {
        id: 11,
        created_at: dt("2024-03-03 09:00:00"),
        content: "mine".into(),
        peer_id: None,
        topic: "chat".into(),
        sent: 1,
        is_direct: 0,
        target_peer: None,
        msg_id: None,
        sent_at: None,
        sender_nickname: Some("Me".into()),
        attachment_json: None,
        reply_to_json: None,
        encrypted: 0,
        deleted_at: None,
        edited_at: None,
    };
    let chat = message_to_chat(msg);
    assert_eq!(chat.sender_nickname.as_deref(), Some("Me"));
    assert!(chat.origin.sent);
}

#[test]
fn test_message_to_chat_empty_content() {
    let msg = Message {
        id: 3,
        created_at: dt("2024-01-01 00:00:00"),
        content: String::new(),
        peer_id: None,
        topic: "chat".into(),
        sent: 0,
        is_direct: 0,
        target_peer: None,
        msg_id: None,
        sent_at: None,
        sender_nickname: None,
        attachment_json: None,
        reply_to_json: None,
        encrypted: 0,
        deleted_at: None,
        edited_at: None,
    };
    let chat = message_to_chat(msg);
    assert_eq!(chat.content, "");
}

#[test]
fn test_message_to_chat_none_sent_at() {
    let msg = Message {
        id: 4,
        created_at: dt("2024-03-01 00:00:00"),
        content: "test".into(),
        peer_id: Some("p1".into()),
        topic: "t".into(),
        sent: 0,
        is_direct: 0,
        target_peer: None,
        msg_id: Some("m4".into()),
        sent_at: None,
        sender_nickname: None,
        attachment_json: None,
        reply_to_json: None,
        encrypted: 0,
        deleted_at: None,
        edited_at: None,
    };
    let chat = message_to_chat(msg);
    assert!(chat.sent_at.is_none());
}

#[test]
fn test_group_message_to_mobile_uses_message_timestamps() {
    // Both chat DTOs must print the same local HH:MM and full creation
    // timestamp for an identical DB row, whichever mapper built them.
    let sent_at = Some(1_718_455_800.0);
    let message = Message {
        id: 7,
        created_at: dt("2024-06-15 10:30:00"),
        content: "hello".into(),
        peer_id: None,
        topic: "chat".into(),
        sent: 1,
        is_direct: 0,
        target_peer: None,
        msg_id: None,
        sent_at,
        sender_nickname: None,
        attachment_json: None,
        reply_to_json: None,
        encrypted: 0,
        deleted_at: None,
        edited_at: None,
    };
    let group_message = GroupMessage {
        id: 8,
        created_at: dt("2024-06-15 10:30:00"),
        group_id: "group-a".into(),
        content: "hello".into(),
        peer_id: None,
        sent: 1,
        msg_id: None,
        reply_to_json: None,
        sent_at,
        sender_nickname: None,
        attachment_json: None,
        deleted_at: None,
        edited_at: None,
    };

    let chat = message_to_chat(message);
    let group = group_message_to_mobile(group_message);
    assert!(chat.sent_at.is_some());
    assert_eq!(group.sent_at, chat.sent_at);
    assert_eq!(group.created_at, chat.created_at);
}

// ── process_event_for_mobile ───────────────────────────────────────

#[test]
fn test_process_event_stores_nickname_from_dm() {
    let ev = SwarmEvent::DirectMessage(MessageEvent {
        content: String::new(),
        peer_id: "peer-nick".into(),
        latency: None,
        nickname: Some("Bob".into()),
        msg_id: None,
        attachment: None,
        reply_to: None,
        encrypted: false,
    });
    let _ = process_event_for_mobile(&ev);
}

#[test]
fn test_process_event_broadcast_with_nickname() {
    let ev = SwarmEvent::BroadcastMessage(MessageEvent {
        content: "hi all".into(),
        peer_id: "peer-broad".into(),
        latency: None,
        nickname: Some("Charlie".into()),
        msg_id: Some("msg-b1".into()),
        attachment: None,
        reply_to: None,
        encrypted: false,
    });
    let _ = process_event_for_mobile(&ev);
}

#[test]
fn test_process_event_logs_file_received() {
    // Unique markers avoid clashing with other tests writing to the shared
    // in-memory log buffer in parallel.
    let _ = process_event_for_mobile(&SwarmEvent::FileReceived {
        peer_id: "peer-file-log".into(),
        file_id: "file-id-log-marker".into(),
        name: "log-marker.bin".into(),
        path: "/tmp/log-marker.bin".into(),
    });
    let logs = crate::logging::get_tui_logs();
    assert!(
            logs.iter().any(|l| l.contains(
                "Received file log-marker.bin (file-id-log-marker) from peer-file-log at /tmp/log-marker.bin"
            )),
            "file_received must land in the shared log buffer: {logs:?}"
        );
}

#[cfg(feature = "mdns")]
#[test]
fn test_process_event_logs_peer_expired() {
    let _ = process_event_for_mobile(&SwarmEvent::PeerExpired {
        peer_id: "peer-expired-log-marker".into(),
    });
    let logs = crate::logging::get_tui_logs();
    assert!(
        logs.iter()
            .any(|l| l.contains("Peer expired: peer-expired-log-marker")),
        "peer_expired must land in the shared log buffer: {logs:?}"
    );
}

#[test]
fn test_process_event_peer_connected_returns_greeting() {
    let cmd = process_event_for_mobile(&SwarmEvent::PeerConnected("p1".into()));
    match cmd {
        Some(SwarmCommand::SendDm { peer_id, .. }) => assert_eq!(peer_id, "p1"),
        other => panic!("PeerConnected must return a nickname greeting, got {other:?}"),
    }
}

#[test]
fn test_process_event_ignored_variants() {
    assert!(process_event_for_mobile(&SwarmEvent::PeerDisconnected("p1".into())).is_none());
    assert!(
        process_event_for_mobile(&SwarmEvent::ListenAddrEstablished(
            "/ip4/0.0.0.0/tcp/0".into()
        ))
        .is_none()
    );
    assert!(
        process_event_for_mobile(&SwarmEvent::Receipt {
            peer_id: "p1".into(),
            ack_for: "m1".into(),
            received_at: None,
        })
        .is_none()
    );
}

#[cfg(feature = "mdns")]
#[test]
fn test_process_event_peer_discovered() {
    let addr: libp2p::Multiaddr = "/ip4/10.0.0.1/tcp/9000".parse().unwrap();
    let ev = SwarmEvent::PeerDiscovered {
        peer_id: "p-disc".into(),
        addresses: vec![addr],
    };
    let _ = process_event_for_mobile(&ev);
}

// ── forward_swarm_events ──────────────────────────────────────────

#[tokio::test]
#[serial(db)]
async fn test_forward_swarm_events_sends_greeting_without_blocking() {
    // Regression: the forwarder runs *inside* a Tokio task. The old
    // implementation called `blocking_send` for the nickname greeting,
    // which panics ("Cannot block the current thread from within a
    // runtime") and killed the whole event pipeline on first connect.
    let guard = crate::db::shared_db_test_lock().lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("forwarder_test.sqlite");
    crate::reset_db_url();
    crate::db::set_db_url(&db_path.to_string_lossy());
    let _ = crate::init_database();
    // `#[serial(db)]` holds the DB namespace across the whole async test;
    // drop the raw guard so it is not held across the awaits below. The
    // forwarder's DB writes are best-effort and not asserted here anyway.
    drop(guard);

    let (event_tx, event_rx) = mpsc::channel(4);
    let (cmd_tx, mut cmd_rx) = mpsc::channel(4);
    let (broadcast_tx, mut broadcast_rx) = tokio::sync::broadcast::channel(4);

    let task = tokio::spawn(forward_swarm_events(event_rx, cmd_tx, broadcast_tx));

    event_tx
        .send(SwarmEvent::PeerConnected("forwarder-peer".into()))
        .await
        .expect("feed event");
    drop(event_tx);

    // The greeting must arrive on the async command channel.
    let cmd = cmd_rx.recv().await.expect("greeting delivered");
    match cmd {
        SwarmCommand::SendDm { peer_id, .. } => assert_eq!(peer_id, "forwarder-peer"),
        other => panic!("expected nickname greeting, got {other:?}"),
    }
    // The same event is fanned out to broadcast consumers.
    let forwarded = broadcast_rx.recv().await.expect("event broadcast");
    assert!(matches!(forwarded, SwarmEvent::PeerConnected(id) if id == "forwarder-peer"));

    task.await.expect("forwarder task completes cleanly");
}

// ── is_nickname_only_dm ───────────────────────────────────────────

#[test]
fn test_is_nickname_only_dm_empty_content_with_nickname() {
    let ev = SwarmEvent::DirectMessage(MessageEvent {
        content: String::new(),
        peer_id: "p1".into(),
        latency: None,
        nickname: Some("Alice".into()),
        msg_id: None,
        attachment: None,
        reply_to: None,
        encrypted: false,
    });
    assert!(is_nickname_only_dm(&ev));
}

#[test]
fn test_is_nickname_only_dm_empty_without_nickname_is_regular_dm() {
    let ev = SwarmEvent::DirectMessage(MessageEvent {
        content: String::new(),
        peer_id: "p1".into(),
        latency: None,
        nickname: None,
        msg_id: None,
        attachment: None,
        reply_to: None,
        encrypted: false,
    });
    assert!(!is_nickname_only_dm(&ev));
}

#[test]
fn test_is_nickname_only_dm_real_content_with_nickname_is_regular_dm() {
    let ev = SwarmEvent::DirectMessage(MessageEvent {
        content: "hello".into(),
        peer_id: "p1".into(),
        latency: None,
        nickname: Some("Alice".into()),
        msg_id: None,
        attachment: None,
        reply_to: None,
        encrypted: false,
    });
    assert!(!is_nickname_only_dm(&ev));
}

#[test]
fn test_is_nickname_only_dm_ignores_empty_broadcasts() {
    let ev = SwarmEvent::BroadcastMessage(MessageEvent {
        content: String::new(),
        peer_id: "p1".into(),
        latency: None,
        nickname: Some("Alice".into()),
        msg_id: None,
        attachment: None,
        reply_to: None,
        encrypted: false,
    });
    assert!(!is_nickname_only_dm(&ev));
}

// ── SwarmEventJson / ChatMessage / MobilePeerRecord ────────────────

#[test]
fn test_swarm_event_json_clone_debug() {
    let j = default_event();
    let cloned = j.clone();
    assert_eq!(cloned.event_type, "");
    let _ = format!("{j:?}");
}

#[test]
fn test_chat_message_clone_debug() {
    let msg = Message {
        id: 1,
        created_at: dt("2024-01-01 00:00:00"),
        content: "test".into(),
        peer_id: None,
        topic: "t".into(),
        sent: 1,
        is_direct: 0,
        target_peer: None,
        msg_id: None,
        sent_at: None,
        sender_nickname: None,
        attachment_json: None,
        reply_to_json: None,
        encrypted: 0,
        deleted_at: None,
        edited_at: None,
    };
    let chat = message_to_chat(msg);
    let cloned = chat.clone();
    assert_eq!(cloned.id, 1);
    let _ = format!("{chat:?}");
}

#[test]
fn test_mobile_peer_record_clone_debug() {
    let r = MobilePeerRecord {
        peer_id: "p1".into(),
        first_seen: "2024-01-01".into(),
        last_seen: "2024-06-01".into(),
        nickname: Some("Alice".into()),
        local_nickname: None,
        display_name: "Alice".into(),
    };
    let cloned = r.clone();
    assert_eq!(cloned.peer_id, "p1");
    let _ = format!("{r:?}");
}

// ── start/stop node error paths ────────────────────────────────────

#[test]
fn test_stop_node_without_start_is_noop() {
    // stop_node should not panic even if NODE was never set
    // (it uses OnceLock::get which returns None)
    let _ = stop_node();
}

#[test]
fn test_get_node_peer_id_without_start_fails() {
    let result = get_node_peer_id();
    assert!(result.is_err());
}

#[test]
fn test_poll_event_without_start_fails() {
    let result = poll_event();
    assert!(result.is_err());
}

#[test]
#[serial(db)]
fn test_load_broadcast_messages_empty_db() {
    let _guard = crate::db::shared_db_test_lock().lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("bc_test.sqlite");
    crate::reset_db_url();
    crate::db::set_db_url(&db_path.to_string_lossy());
    let _ = crate::init_database();

    let msgs = load_broadcast_messages(100).expect("load broadcast");
    assert!(msgs.is_empty());
}

#[test]
#[serial(db)]
fn test_load_dm_messages_empty_db() {
    let _guard = crate::db::shared_db_test_lock().lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("dm_test.sqlite");
    crate::reset_db_url();
    crate::db::set_db_url(&db_path.to_string_lossy());
    let _ = crate::init_database();

    let msgs = load_dm_messages("peer1".into(), 100).expect("load dm");
    assert!(msgs.is_empty());
}

// ── confirmed (delivery receipt) flags ─────────────────────────────

fn save_outgoing_with_msg_id(content: &str, msg_id: &str, is_direct: bool, target: Option<&str>) {
    let saved = save_message_with_meta(
        content,
        None,
        CHAT_TOPIC,
        is_direct,
        target,
        MessageMeta {
            msg_id: Some(msg_id.to_string()),
            sent_at: Some(1.0),
            ..MessageMeta::default()
        },
    )
    .expect("save message");
    mark_message_sent(saved.id).expect("mark sent");
}

#[test]
#[serial(db)]
fn test_load_broadcast_messages_marks_confirmed_receipts() {
    let _guard = crate::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("bc_confirmed.sqlite");
    crate::reset_db_url();
    crate::db::set_db_url(&db_path.to_string_lossy());
    crate::init_database().expect("init db");

    // Two outgoing broadcasts; only the first gets a receipt. An incoming
    // row carries a receipt id too, and another echoes the *same* id as the
    // acknowledged outgoing message: neither ever shows a delivery tick.
    save_outgoing_with_msg_id("acked", "msg-acked", false, None);
    save_outgoing_with_msg_id("unacked", "msg-unacked", false, None);
    let inbound = save_message_with_meta(
        "in",
        Some("peer-a"),
        CHAT_TOPIC,
        false,
        None,
        MessageMeta {
            msg_id: Some("msg-in".to_string()),
            sent_at: Some(3.0),
            ..MessageMeta::default()
        },
    )
    .expect("save inbound");
    let echoed = save_message_with_meta(
        "echo",
        Some("peer-a"),
        CHAT_TOPIC,
        false,
        None,
        MessageMeta {
            msg_id: Some("msg-acked".to_string()),
            sent_at: Some(4.0),
            ..MessageMeta::default()
        },
    )
    .expect("save echoed inbound");

    crate::messages::save_receipt("msg-acked", "peer-a", 0, 5.0).expect("receipt");
    crate::messages::save_receipt("msg-in", "peer-a", 0, 6.0).expect("inbound receipt");

    let msgs = load_broadcast_messages(100).expect("load broadcast");
    assert_eq!(msgs.len(), 4);
    let by_id = |id: &str, sent: bool| {
        msgs.iter()
            .find(|m| m.msg_id.as_deref() == Some(id) && m.origin.sent == sent)
            .expect("message present")
    };
    assert!(
        by_id("msg-acked", true).confirmed,
        "acked message shows a tick"
    );
    assert!(
        !by_id("msg-unacked", true).confirmed,
        "unacked message has no tick"
    );
    assert!(
        !by_id("msg-in", false).confirmed,
        "inbound message never ticks"
    );
    assert!(
        !by_id("msg-acked", false).confirmed,
        "an inbound echo of our wire id never ticks"
    );
    assert_eq!(inbound.sent, 0);
    assert_eq!(echoed.sent, 0);
}

#[test]
#[serial(db)]
fn test_load_dm_messages_marks_confirmed_receipts() {
    let _guard = crate::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("dm_confirmed.sqlite");
    crate::reset_db_url();
    crate::db::set_db_url(&db_path.to_string_lossy());
    crate::init_database().expect("init db");

    save_outgoing_with_msg_id("hi", "dm-acked", true, Some("peer-b"));
    crate::messages::save_receipt("dm-acked", "peer-b", 1, 7.0).expect("receipt");

    let msgs = load_dm_messages("peer-b".into(), 100).expect("load dm");
    let dm = msgs
        .iter()
        .find(|m| m.msg_id.as_deref() == Some("dm-acked"))
        .expect("dm present");
    assert!(dm.confirmed, "acknowledged DM shows a tick");
}
