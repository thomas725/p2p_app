use libp2p::{
    Multiaddr,
    futures::StreamExt as _,
    gossipsub, identity,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::Duration,
};
use tokio::time::timeout;

const TEST_TOPIC: &str = "test-integration";

#[derive(NetworkBehaviour)]
struct TestBehaviour {
    gossipsub: gossipsub::Behaviour,
}

struct TestNode {
    swarm: libp2p::Swarm<TestBehaviour>,
    peer_id: libp2p::PeerId,
    listen_addr: Option<Multiaddr>,
}

async fn create_node() -> Result<TestNode, Box<dyn std::error::Error>> {
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = keypair.public().to_peer_id();

    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(1))
        .validation_mode(gossipsub::ValidationMode::Permissive)
        .message_id_fn(message_id_fn)
        .build()
        .map_err(|msg| std::io::Error::new(std::io::ErrorKind::Other, msg))?;

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(keypair.clone()),
        gossipsub_config,
    )?;

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            libp2p::noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|_| Ok(TestBehaviour { gossipsub }))?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    let topic = gossipsub::IdentTopic::new(TEST_TOPIC);
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    Ok(TestNode {
        swarm,
        peer_id,
        listen_addr: None,
    })
}

async fn connect_nodes(
    node_a: &mut TestNode,
    node_b: &mut TestNode,
) -> Result<(), Box<dyn std::error::Error>> {
    node_a.swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?;
    node_b.swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?;

    let mut a_connected = false;
    let mut b_connected = false;
    let deadline = Duration::from_secs(15);

    let peer_a = node_a.peer_id;
    let peer_b = node_b.peer_id;

    let _ = timeout(deadline, async {
        loop {
            tokio::select! {
                event = node_a.swarm.select_next_some() => {
                    if let SwarmEvent::NewListenAddr { ref address, .. } = event {
                        if node_a.listen_addr.is_none() {
                            node_a.listen_addr = Some(address.clone());
                            if let Some(ref addr_b) = node_b.listen_addr {
                                let _ = node_a.swarm.dial(addr_b.clone());
                            }
                        }
                    }
                    if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event {
                        if peer_id == peer_b {
                            a_connected = true;
                        }
                    }
                }
                event = node_b.swarm.select_next_some() => {
                    if let SwarmEvent::NewListenAddr { ref address, .. } = event {
                        if node_b.listen_addr.is_none() {
                            node_b.listen_addr = Some(address.clone());
                            if let Some(ref addr_a) = node_a.listen_addr {
                                let _ = node_b.swarm.dial(addr_a.clone());
                            }
                        }
                    }
                    if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event {
                        if peer_id == peer_a {
                            b_connected = true;
                        }
                    }
                }
            }

            if a_connected && b_connected {
                break;
            }
        }
    })
    .await;

    if !a_connected || !b_connected {
        return Err("Failed to establish connection".into());
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    node_a
        .swarm
        .behaviour_mut()
        .gossipsub
        .add_explicit_peer(&node_b.peer_id);
    node_b
        .swarm
        .behaviour_mut()
        .gossipsub
        .add_explicit_peer(&node_a.peer_id);

    let mut subscribed_a = false;
    let mut subscribed_b = false;

    let subscribe_deadline = Duration::from_secs(5);
    let _ = timeout(subscribe_deadline, async {
        loop {
            tokio::select! {
                event = node_a.swarm.select_next_some() => {
                    if let SwarmEvent::Behaviour(TestBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, .. })) = event {
                        if peer_id == peer_b {
                            subscribed_a = true;
                        }
                    }
                }
                event = node_b.swarm.select_next_some() => {
                    if let SwarmEvent::Behaviour(TestBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, .. })) = event {
                        if peer_id == peer_a {
                            subscribed_b = true;
                        }
                    }
                }
            }
            if subscribed_a && subscribed_b {
                break;
            }
        }
    }).await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}

#[tokio::test]
async fn test_p2p_message_transfer() -> Result<(), Box<dyn std::error::Error>> {
    let mut node_a = create_node().await?;
    let mut node_b = create_node().await?;

    connect_nodes(&mut node_a, &mut node_b).await?;

    let topic = gossipsub::IdentTopic::new(TEST_TOPIC);
    let message = b"Hello from node A";

    node_a
        .swarm
        .behaviour_mut()
        .gossipsub
        .publish(topic.clone(), message)?;

    let received = timeout(Duration::from_secs(5), async {
        loop {
            let event = node_b.swarm.select_next_some().await;
            if let SwarmEvent::Behaviour(TestBehaviourEvent::Gossipsub(
                gossipsub::Event::Message { message, .. },
            )) = event
            {
                break String::from_utf8_lossy(&message.data).to_string();
            }
        }
    })
    .await
    .map_err(|_| "Timeout waiting for message")?;

    assert_eq!(received, "Hello from node A");

    Ok(())
}

#[tokio::test]
async fn test_bidirectional_messages() -> Result<(), Box<dyn std::error::Error>> {
    let mut node_a = create_node().await?;
    let mut node_b = create_node().await?;

    connect_nodes(&mut node_a, &mut node_b).await?;

    let topic = gossipsub::IdentTopic::new(TEST_TOPIC);

    let msg_a = b"Message from A";
    let msg_b = b"Message from B";

    node_a
        .swarm
        .behaviour_mut()
        .gossipsub
        .publish(topic.clone(), msg_a)?;

    node_b
        .swarm
        .behaviour_mut()
        .gossipsub
        .publish(topic.clone(), msg_b)?;

    let mut messages = Vec::new();
    let deadline = Duration::from_secs(5);

    let _ = timeout(deadline, async {
        loop {
            tokio::select! {
                event = node_a.swarm.select_next_some() => {
                    if let SwarmEvent::Behaviour(TestBehaviourEvent::Gossipsub(
                        gossipsub::Event::Message { message, .. })
                    ) = event {
                        messages.push(String::from_utf8_lossy(&message.data).to_string());
                    }
                }
                event = node_b.swarm.select_next_some() => {
                    if let SwarmEvent::Behaviour(TestBehaviourEvent::Gossipsub(
                        gossipsub::Event::Message { message, .. })
                    ) = event {
                        messages.push(String::from_utf8_lossy(&message.data).to_string());
                    }
                }
            }
            if messages.len() >= 2 {
                break;
            }
        }
    })
    .await;

    assert!(messages.contains(&"Message from A".to_string()));
    assert!(messages.contains(&"Message from B".to_string()));

    Ok(())
}
