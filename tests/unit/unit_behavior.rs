use super::*;

#[test]
fn constants_are_stable() {
    assert_eq!(DM_PROTOCOL_NAME, "/p2p-chat/dm/1.0.0");
    assert_eq!(CHAT_TOPIC, "test-net");
}

#[test]
fn direct_message_default_is_empty() {
    let dm = DirectMessage::default();
    assert!(dm.content.is_empty());
    assert_eq!(dm.timestamp, 0);
    assert!(dm.sent_at.is_none());
    assert!(dm.nickname.is_none());
    assert!(dm.msg_id.is_none());
    assert!(dm.ack_for.is_none());
    assert!(dm.received_at.is_none());
}

#[test]
fn broadcast_message_default_is_empty() {
    let msg = BroadcastMessage::default();
    assert!(msg.content.is_empty());
    assert!(msg.sent_at.is_none());
    assert!(msg.nickname.is_none());
    assert!(msg.msg_id.is_none());
}

#[test]
fn direct_message_json_roundtrip() {
    let dm = DirectMessage {
        content: "hello".to_string(),
        timestamp: 123,
        sent_at: Some(1.2),
        nickname: Some("nick".to_string()),
        msg_id: Some("m1".to_string()),
        ack_for: Some("ack".to_string()),
        received_at: Some(3.4),
    };
    let json = serde_json::to_string(&dm).expect("serialize");
    let parsed: DirectMessage = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.content, "hello");
    assert_eq!(parsed.nickname.as_deref(), Some("nick"));
    assert_eq!(parsed.msg_id.as_deref(), Some("m1"));
    assert_eq!(parsed.ack_for.as_deref(), Some("ack"));
}

#[test]
fn build_behaviour_for_all_network_sizes() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let key = libp2p_identity::Keypair::generate_ed25519();
        let _small = build_behaviour(&key, NetworkSize::Small);
        let _medium = build_behaviour(&key, NetworkSize::Medium);
        let _large = build_behaviour(&key, NetworkSize::Large);
    });
}
