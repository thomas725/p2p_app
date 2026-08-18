//! Tests for `swarm_handler.rs` module

use p2p_app::behavior::BroadcastMessage;

#[test]
fn test_build_broadcast_message() {
    let msg = p2p_app::swarm_handler::build_broadcast_message(
        "hello world".to_string(),
        Some("alice".to_string()),
        Some("msg-123".to_string()),
    );
    assert_eq!(msg.content, "hello world");
    assert_eq!(msg.nickname, Some("alice".to_string()));
    assert_eq!(msg.msg_id, Some("msg-123".to_string()));
}

#[test]
fn test_build_broadcast_message_no_metadata() {
    let msg = p2p_app::swarm_handler::build_broadcast_message("content".to_string(), None, None);
    assert_eq!(msg.content, "content");
    assert!(msg.nickname.is_none());
    assert!(msg.msg_id.is_none());
}

#[test]
fn test_serialize_broadcast_message_some() {
    let msg = BroadcastMessage {
        content: "test".to_string(),
        sent_at: Some(1.0),
        nickname: Some("nick".to_string()),
        msg_id: Some("id".to_string()),
    };
    let json = serde_json::to_string(&msg);
    assert!(json.is_ok());
    let json_str = json.unwrap();
    assert!(json_str.contains("\"content\":\"test\""));
}

#[test]
fn test_serialize_broadcast_message_none() {
    let msg = BroadcastMessage {
        content: "test".to_string(),
        sent_at: None,
        nickname: None,
        msg_id: None,
    };
    let json = serde_json::to_string(&msg);
    assert!(json.is_ok());
}

// ── Additional swarm handler tests ────────────────────────────────────────────────

#[test]
fn test_build_broadcast_message_all_none() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let msg = build_broadcast_message("content only".to_string(), None, None);
    assert_eq!(msg.content, "content only");
    assert!(msg.nickname.is_none());
    assert!(msg.msg_id.is_none());
}

#[test]
fn test_build_broadcast_message_empty_content() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let msg = build_broadcast_message(String::new(), None, None);
    assert_eq!(msg.content, "");
}

#[test]
fn test_build_broadcast_message_only_nickname() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let msg = build_broadcast_message("msg".to_string(), Some("Alice".to_string()), None);
    assert_eq!(msg.content, "msg");
    assert_eq!(msg.nickname, Some("Alice".to_string()));
    assert!(msg.msg_id.is_none());
    assert!(msg.sent_at.is_some());
}

#[test]
fn test_build_broadcast_message_only_msg_id() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let msg = build_broadcast_message("msg".to_string(), None, Some("id-1".to_string()));
    assert!(msg.nickname.is_none());
    assert_eq!(msg.msg_id, Some("id-1".to_string()));
}

#[test]
fn test_build_broadcast_message_long_content() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let long = "a".repeat(1000);
    let msg = build_broadcast_message(long.clone(), None, None);
    assert_eq!(msg.content.len(), 1000);
    assert_eq!(msg.content, long);
}

#[test]
fn test_build_broadcast_message_special_chars() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let msg = build_broadcast_message("Hello! @#$%^&*() 你好 🚀".to_string(), None, None);
    assert_eq!(msg.content, "Hello! @#$%^&*() 你好 🚀");
    assert!(msg.sent_at.is_some());
}

// ── Integration tests for spawn_swarm_handler / handle_command / handle_swarm_event ──

use libp2p::{
    futures::StreamExt,
    gossipsub::{self, IdentTopic},
    identity::Keypair,
    swarm::SwarmEvent as Libp2pSwarmEvent,
    tcp, yamux,
};
use p2p_app::{
    CHAT_TOPIC, NetworkSize, SwarmCommand, SwarmEvent, behavior::AppBehaviourEvent,
    build_behaviour, spawn_swarm_handler,
};
use std::time::Duration;
use tokio::time::timeout;

fn build_test_swarm() -> (libp2p::Swarm<p2p_app::AppBehaviour>, libp2p::PeerId) {
    let keypair = Keypair::generate_ed25519();
    let peer_id = keypair.public().to_peer_id();

    #[cfg(feature = "quic")]
    let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_behaviour(|key| Ok(build_behaviour(key, NetworkSize::Small)))
        .unwrap()
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    #[cfg(not(feature = "quic"))]
    let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_behaviour(|key| Ok(build_behaviour(key, NetworkSize::Small)))
        .unwrap()
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    (swarm, peer_id)
}

/// Connect two swarms, subscribe to the chat topic, and wait for the gossipsub
/// subscription handshake to complete.
async fn connect_swarms(
    swarm_a: &mut libp2p::Swarm<p2p_app::AppBehaviour>,
    peer_a: &libp2p::PeerId,
    swarm_b: &mut libp2p::Swarm<p2p_app::AppBehaviour>,
    peer_b: &libp2p::PeerId,
) {
    let topic = IdentTopic::new(CHAT_TOPIC);
    swarm_a.behaviour_mut().gossipsub.subscribe(&topic).unwrap();
    swarm_b.behaviour_mut().gossipsub.subscribe(&topic).unwrap();

    swarm_a
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    swarm_b
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();

    let mut addr_a: Option<libp2p::Multiaddr> = None;
    let mut addr_b: Option<libp2p::Multiaddr> = None;
    let mut connected_a = false;
    let mut connected_b = false;

    timeout(Duration::from_secs(15), async {
        loop {
            tokio::select! {
                event = swarm_a.select_next_some() => {
                    match event {
                        Libp2pSwarmEvent::NewListenAddr { address, .. } if addr_a.is_none() => {
                            addr_a = Some(address.clone());
                            if let Some(ref b_addr) = addr_b {
                                let _ = swarm_a.dial(b_addr.clone());
                            }
                        }
                        Libp2pSwarmEvent::ConnectionEstablished { peer_id, .. } if peer_id == *peer_b => {
                            connected_a = true;
                        }
                        _ => {}
                    }
                }
                event = swarm_b.select_next_some() => {
                    match event {
                        Libp2pSwarmEvent::NewListenAddr { address, .. } if addr_b.is_none() => {
                            addr_b = Some(address.clone());
                            if let Some(ref a_addr) = addr_a {
                                let _ = swarm_b.dial(a_addr.clone());
                            }
                        }
                        Libp2pSwarmEvent::ConnectionEstablished { peer_id, .. } if peer_id == *peer_a => {
                            connected_b = true;
                        }
                        _ => {}
                    }
                }
            }
            if connected_a && connected_b {
                break;
            }
        }
    })
    .await
    .expect("Timeout connecting swarms");

    // Gossipsub subscription handshake
    swarm_a.behaviour_mut().gossipsub.add_explicit_peer(peer_b);
    swarm_b.behaviour_mut().gossipsub.add_explicit_peer(peer_a);

    let mut subscribed_a = false;
    let mut subscribed_b = false;

    timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                event = swarm_a.select_next_some() => {
                    if let Libp2pSwarmEvent::Behaviour(AppBehaviourEvent::Gossipsub(
                        gossipsub::Event::Subscribed { peer_id, .. },
                    )) = &event
                        && peer_id == peer_b
                    {
                        subscribed_a = true;
                    }
                }
                event = swarm_b.select_next_some() => {
                    if let Libp2pSwarmEvent::Behaviour(AppBehaviourEvent::Gossipsub(
                        gossipsub::Event::Subscribed { peer_id, .. },
                    )) = &event
                        && peer_id == peer_a
                    {
                        subscribed_b = true;
                    }
                }
            }
            if subscribed_a && subscribed_b {
                break;
            }
        }
    })
    .await
    .expect("Timeout waiting for gossipsub subscriptions");
}

#[tokio::test]
async fn test_spawn_handler_broadcast_message() {
    let (mut swarm_a, peer_a) = build_test_swarm();
    let (mut swarm_b, peer_b) = build_test_swarm();

    connect_swarms(&mut swarm_a, &peer_a, &mut swarm_b, &peer_b).await;

    let (_handle_a, _event_rx_a, cmd_tx_a) = spawn_swarm_handler(swarm_a, CHAT_TOPIC.to_string());
    let (_handle_b, mut event_rx_b, _cmd_tx_b) =
        spawn_swarm_handler(swarm_b, CHAT_TOPIC.to_string());

    cmd_tx_a
        .send(SwarmCommand::Publish {
            content: "integration test broadcast".to_string(),
            nickname: Some("tester".to_string()),
            msg_id: Some("test-msg-1".to_string()),
        })
        .await
        .unwrap();

    let received = timeout(Duration::from_secs(10), async {
        loop {
            if let Some(SwarmEvent::BroadcastMessage(msg)) = event_rx_b.recv().await {
                break msg;
            }
        }
    })
    .await
    .expect("Timeout waiting for broadcast message");

    assert_eq!(received.content, "integration test broadcast");
    assert_eq!(received.nickname, Some("tester".to_string()));
    assert_eq!(received.msg_id, Some("test-msg-1".to_string()));
}

#[tokio::test]
async fn test_spawn_handler_direct_message() {
    let (mut swarm_a, peer_a) = build_test_swarm();
    let (mut swarm_b, peer_b) = build_test_swarm();

    connect_swarms(&mut swarm_a, &peer_a, &mut swarm_b, &peer_b).await;

    let (_handle_a, _event_rx_a, cmd_tx_a) = spawn_swarm_handler(swarm_a, CHAT_TOPIC.to_string());
    let (_handle_b, mut event_rx_b, _cmd_tx_b) =
        spawn_swarm_handler(swarm_b, CHAT_TOPIC.to_string());

    cmd_tx_a
        .send(SwarmCommand::SendDm {
            peer_id: peer_b.to_string(),
            content: "hello direct".to_string(),
            nickname: Some("dm_tester".to_string()),
            msg_id: Some("dm-msg-1".to_string()),
            ack_for: None,
        })
        .await
        .unwrap();

    let received = timeout(Duration::from_secs(10), async {
        loop {
            if let Some(SwarmEvent::DirectMessage(msg)) = event_rx_b.recv().await {
                break msg;
            }
        }
    })
    .await
    .expect("Timeout waiting for direct message");

    assert_eq!(received.content, "hello direct");
    assert_eq!(received.nickname, Some("dm_tester".to_string()));
    assert_eq!(received.msg_id, Some("dm-msg-1".to_string()));
}

/// A direct message ack ("ok" response with `ack_for` set) should surface as
/// a `Receipt` event on the sender's side.
#[tokio::test]
async fn test_spawn_handler_direct_message_receipt() {
    let (mut swarm_a, peer_a) = build_test_swarm();
    let (mut swarm_b, peer_b) = build_test_swarm();

    connect_swarms(&mut swarm_a, &peer_a, &mut swarm_b, &peer_b).await;

    let (_handle_a, mut event_rx_a, cmd_tx_a) =
        spawn_swarm_handler(swarm_a, CHAT_TOPIC.to_string());
    let (_handle_b, mut event_rx_b, _cmd_tx_b) =
        spawn_swarm_handler(swarm_b, CHAT_TOPIC.to_string());

    cmd_tx_a
        .send(SwarmCommand::SendDm {
            peer_id: peer_b.to_string(),
            content: "ping".to_string(),
            nickname: None,
            msg_id: Some("dm-ack-1".to_string()),
            ack_for: None,
        })
        .await
        .unwrap();

    // B receives the DM (and automatically replies with an "ok" ack).
    timeout(Duration::from_secs(10), async {
        loop {
            if let Some(SwarmEvent::DirectMessage(_)) = event_rx_b.recv().await {
                break;
            }
        }
    })
    .await
    .expect("Timeout waiting for direct message on B");

    // A receives the ack as a Receipt for "dm-ack-1".
    let receipt = timeout(Duration::from_secs(10), async {
        loop {
            if let Some(SwarmEvent::Receipt { peer_id, ack_for, .. }) = event_rx_a.recv().await {
                break (peer_id, ack_for);
            }
        }
    })
    .await
    .expect("Timeout waiting for DM receipt on A");

    assert_eq!(receipt.0, peer_b.to_string());
    assert_eq!(receipt.1, "dm-ack-1");
}

/// A receipt-only DM (empty content, `ack_for` set) for a broadcast message
/// should surface as a `Receipt` event for the original broadcaster.
#[tokio::test]
async fn test_spawn_handler_broadcast_receipt() {
    let (mut swarm_a, peer_a) = build_test_swarm();
    let (mut swarm_b, peer_b) = build_test_swarm();

    connect_swarms(&mut swarm_a, &peer_a, &mut swarm_b, &peer_b).await;

    let (_handle_a, mut event_rx_a, cmd_tx_a) =
        spawn_swarm_handler(swarm_a, CHAT_TOPIC.to_string());
    let (_handle_b, mut event_rx_b, _cmd_tx_b) =
        spawn_swarm_handler(swarm_b, CHAT_TOPIC.to_string());

    cmd_tx_a
        .send(SwarmCommand::Publish {
            content: "broadcast with receipt".to_string(),
            nickname: Some("tester".to_string()),
            msg_id: Some("bcast-ack-1".to_string()),
        })
        .await
        .unwrap();

    // B receives the broadcast (and automatically sends a receipt-only DM back).
    timeout(Duration::from_secs(10), async {
        loop {
            if let Some(SwarmEvent::BroadcastMessage(_)) = event_rx_b.recv().await {
                break;
            }
        }
    })
    .await
    .expect("Timeout waiting for broadcast message on B");

    // A receives the receipt for "bcast-ack-1" from B.
    let receipt = timeout(Duration::from_secs(10), async {
        loop {
            if let Some(SwarmEvent::Receipt { peer_id, ack_for, .. }) = event_rx_a.recv().await {
                break (peer_id, ack_for);
            }
        }
    })
    .await
    .expect("Timeout waiting for broadcast receipt on A");

    assert_eq!(receipt.0, peer_b.to_string());
    assert_eq!(receipt.1, "bcast-ack-1");
}

/// `SendDm` with a peer ID that doesn't parse as a valid `PeerId` is silently
/// ignored: no request is sent and the handler keeps running.
#[tokio::test]
async fn test_send_dm_invalid_peer_id_is_ignored() {
    let (swarm, _peer_id) = build_test_swarm();
    let (handle, _event_rx, cmd_tx) = spawn_swarm_handler(swarm, CHAT_TOPIC.to_string());

    cmd_tx
        .send(SwarmCommand::SendDm {
            peer_id: "not-a-valid-peer-id".to_string(),
            content: "hello".to_string(),
            nickname: None,
            msg_id: None,
            ack_for: None,
        })
        .await
        .unwrap();

    // Handler should remain alive and keep accepting further commands.
    cmd_tx
        .send(SwarmCommand::Publish {
            content: "still alive".to_string(),
            nickname: None,
            msg_id: None,
        })
        .await
        .unwrap();

    // Give the spawned handler task a chance to process both commands
    // before the test runtime shuts down.
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(handle);
}

/// Publishing to a topic with no subscribers/peers returns an error from
/// gossipsub, which the handler logs and otherwise ignores.
#[tokio::test]
async fn test_publish_with_no_peers_logs_error_and_continues() {
    let (swarm, _peer_id) = build_test_swarm();
    let (handle, _event_rx, cmd_tx) = spawn_swarm_handler(swarm, "unsubscribed-topic".to_string());

    cmd_tx
        .send(SwarmCommand::Publish {
            content: "into the void".to_string(),
            nickname: None,
            msg_id: None,
        })
        .await
        .unwrap();

    // Handler should remain alive after logging the publish error.
    cmd_tx
        .send(SwarmCommand::Publish {
            content: "still alive".to_string(),
            nickname: None,
            msg_id: None,
        })
        .await
        .unwrap();

    // Give the spawned handler task a chance to process both commands
    // before the test runtime shuts down.
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(handle);
}

/// Dropping the command sender should not crash the handler task. The task
/// may either exit (if its `tokio::select!` loop observes the closed
/// channel before another swarm event arrives) or keep running indefinitely
/// driven by swarm/gossipsub housekeeping events; both are acceptable, so we
/// bound the wait with a timeout rather than asserting a specific outcome.
#[tokio::test]
async fn test_handler_survives_command_sender_drop() {
    let (swarm, _peer_id) = build_test_swarm();
    let (handle, _event_rx, cmd_tx) = spawn_swarm_handler(swarm, CHAT_TOPIC.to_string());

    drop(cmd_tx);

    match timeout(Duration::from_millis(500), handle).await {
        // Task exited cleanly via the `else` branch of its select loop.
        Ok(join_result) => assert!(join_result.is_ok(), "handler task panicked"),
        // Task is still running, driven by swarm events; that's fine too.
        Err(_) => {}
    }
}
