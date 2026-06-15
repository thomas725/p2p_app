//! Tests for behavior.rs module via libp2p behavior

mod test_utils;
use serial_test::serial;
use test_utils::with_test_db;

#[test]
fn test_behavior_constants() {
    assert_eq!(p2p_app::behavior::DM_PROTOCOL_NAME, "/p2p-chat/dm/1.0.0");
    assert_eq!(p2p_app::behavior::CHAT_TOPIC, "test-net");
}

#[test]
fn test_direct_message_default() {
    let dm = p2p_app::behavior::DirectMessage::default();
    assert!(dm.content.is_empty());
    assert_eq!(dm.timestamp, 0);
}

#[test]
fn test_broadcast_message_default() {
    let bm = p2p_app::behavior::BroadcastMessage::default();
    assert!(bm.content.is_empty());
}

#[test]
fn test_direct_message_serialization() {
    use serde_json;
    let dm = p2p_app::behavior::DirectMessage {
        content: "hello".to_string(),
        timestamp: 12345,
        sent_at: Some(1234567890.0),
        nickname: Some("Alice".to_string()),
        msg_id: Some("msg-1".to_string()),
        ack_for: None,
        received_at: None,
    };
    let json = serde_json::to_string(&dm).unwrap();
    assert!(json.contains("hello"));
}

#[test]
fn test_broadcast_message_serialization() {
    use serde_json;
    let bm = p2p_app::behavior::BroadcastMessage {
        content: "broadcast test".to_string(),
        sent_at: Some(1234567890.0),
        nickname: Some("Bob".to_string()),
        msg_id: Some("bcast-1".to_string()),
    };
    let json = serde_json::to_string(&bm).unwrap();
    assert!(json.contains("broadcast test"));
}

#[serial]
#[test]
fn test_build_behaviour_creates_app_behaviour() {
    with_test_db(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let keypair = p2p_app::get_libp2p_identity().unwrap();
                let _behaviour =
                    p2p_app::build_behaviour(&keypair, p2p_app::network::NetworkSize::Small);
            });
    });
}

#[serial]
#[test]
fn test_build_behaviour_medium_network() {
    with_test_db(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let keypair = p2p_app::get_libp2p_identity().unwrap();
                let _behaviour =
                    p2p_app::build_behaviour(&keypair, p2p_app::network::NetworkSize::Medium);
            });
    });
}

#[serial]
#[test]
fn test_build_behaviour_large_network() {
    with_test_db(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let keypair = p2p_app::get_libp2p_identity().unwrap();
                let _behaviour =
                    p2p_app::build_behaviour(&keypair, p2p_app::network::NetworkSize::Large);
            });
    });
}

#[test]
fn test_direct_message_fields() {
    let dm = p2p_app::behavior::DirectMessage {
        content: "test".to_string(),
        timestamp: 99,
        sent_at: Some(1.0),
        nickname: Some("nick".to_string()),
        msg_id: Some("id".to_string()),
        ack_for: Some("orig".to_string()),
        received_at: Some(2.0),
    };
    assert_eq!(dm.content, "test");
    assert_eq!(dm.timestamp, 99);
    assert_eq!(dm.ack_for.as_deref(), Some("orig"));
    assert_eq!(dm.received_at, Some(2.0));
}

#[test]
fn test_broadcast_message_fields() {
    let bm = p2p_app::behavior::BroadcastMessage {
        content: "hello".to_string(),
        sent_at: Some(3.0),
        nickname: None,
        msg_id: Some("bm-id".to_string()),
    };
    assert_eq!(bm.content, "hello");
    assert!(bm.nickname.is_none());
}

#[serial]
#[test]
fn test_direct_message_clone() {
    let dm = p2p_app::behavior::DirectMessage {
        content: "test".to_string(),
        timestamp: 100,
        sent_at: Some(1.0),
        nickname: Some("nick".to_string()),
        msg_id: Some("id".to_string()),
        ack_for: None,
        received_at: Some(2.0),
    };
    let cloned = dm.clone();
    assert_eq!(dm.content, cloned.content);
    assert_eq!(dm.timestamp, cloned.timestamp);
}

#[serial]
#[test]
fn test_broadcast_message_clone() {
    let bm = p2p_app::behavior::BroadcastMessage {
        content: "broadcast".to_string(),
        sent_at: Some(1.5),
        nickname: Some("sender".to_string()),
        msg_id: Some("b-id".to_string()),
    };
    let cloned = bm.clone();
    assert_eq!(bm.content, cloned.content);
}

#[serial]
#[test]
fn test_build_behaviour_returns_valid_behaviour() {
    with_test_db(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let keypair = p2p_app::get_libp2p_identity().unwrap();
                let _behaviour =
                    p2p_app::build_behaviour(&keypair, p2p_app::network::NetworkSize::Small);
            });
    });
}

#[serial]
#[test]
fn test_build_swarm_small_network() {
    with_test_db(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let result = p2p_app::behavior::build_swarm(p2p_app::network::NetworkSize::Small);
        assert!(result.is_ok());
    });
}

#[test]
fn test_direct_message_serialization_roundtrip() {
    use p2p_app::behavior::DirectMessage;
    let dm = DirectMessage {
        content: "roundtrip".to_string(),
        timestamp: 999,
        sent_at: Some(1.5),
        nickname: Some("Bob".to_string()),
        msg_id: Some("dm-42".to_string()),
        ack_for: Some("orig-7".to_string()),
        received_at: Some(3.0),
    };
    let json = serde_json::to_string(&dm).unwrap();
    let parsed: DirectMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.content, "roundtrip");
    assert_eq!(parsed.timestamp, 999);
    assert_eq!(parsed.nickname.as_deref(), Some("Bob"));
    assert_eq!(parsed.ack_for.as_deref(), Some("orig-7"));
}
