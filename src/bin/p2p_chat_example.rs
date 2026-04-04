#[cfg(feature = "mdns")]
use libp2p::mdns;
use libp2p::{
    Multiaddr,
    futures::StreamExt as _,
    gossipsub, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use p2p_app::get_libp2p_identity;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::Duration,
};
use tokio::{io, io::AsyncBufReadExt};

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    #[cfg(feature = "mdns")]
    mdns: mdns::tokio::Behaviour,
}

fn build_behaviour_impl(key: &libp2p_identity::Keypair) -> MyBehaviour {
    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .build()
        .expect("gossipsub config should be valid");

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(key.clone()),
        gossipsub_config,
    )
    .expect("gossipsub should be created");

    #[cfg(feature = "mdns")]
    let mut mdns_config = mdns::Config::default();
    mdns_config.query_interval = Duration::from_secs(1);
    let mdns = mdns::tokio::Behaviour::new(mdns_config, key.public().to_peer_id())
        .expect("mDNS should be created");
    MyBehaviour {
        gossipsub,
        #[cfg(feature = "mdns")]
        mdns,
    }
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // libp2p uses the tracing library which helps to understand complex async flows
    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init()
        .map_err(|e| println!("failed to init tracing subscriber: {e}"))
        .ok();

    let mut swarm = {
        let base = libp2p::SwarmBuilder::with_existing_identity(get_libp2p_identity()?)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?;

        #[cfg(feature = "quic")]
        let swarm = base
            .with_quic()
            .with_behaviour(|key| Ok(build_behaviour_impl(key)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        #[cfg(not(feature = "quic"))]
        let swarm = base
            .with_behaviour(|key| Ok(build_behaviour_impl(key)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        swarm
    };

    // Create a Gossipsub topic
    let topic = gossipsub::IdentTopic::new("test-net");
    // subscribes to our topic
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    #[cfg(feature = "quic")]
    {
        swarm
            .listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)
            .map_err(|e| {
                #[cfg(feature = "tracing")]
                tracing::warn!("failed to listen to quic: {e}");
            })
            .ok();
    }
    swarm
        .listen_on("/ip4/0.0.0.0/tcp/0".parse()?)
        .map_err(|e| {
            #[cfg(feature = "tracing")]
            tracing::warn!("failed to listen to tcp: {e}");
        })
        .ok();

    println!("Enter messages via STDIN and they will be sent to connected peers using Gossipsub");
    println!("Or connect to another peer manually: /connect <multiaddr>");

    // Kick it off
    loop {
        tokio::select! {
            Ok(Some(line)) = stdin.next_line() => {
                let line = line.trim();
                if line.starts_with("/connect ") {
                    let addr = line.trim_start_matches("/connect ");
                    match addr.parse::<Multiaddr>() {
                        Ok(multiaddr) => {
                            println!("Dialing {multiaddr}...");
                            swarm.dial(multiaddr).map_err(|e| println!("Dial error: {e:?}")).ok();
                        }
                        Err(e) => println!("Invalid address: {e}"),
                    }
                } else if line.is_empty() {
                    continue;
                } else {
                    swarm
                        .behaviour_mut()
                        .gossipsub
                        .publish(topic.clone(), line.as_bytes())
                        .map_err(|e| println!("Publish error: {e:?}"))
                        .ok();
                }
            }
            event = swarm.select_next_some() => match event {
                #[cfg(feature = "mdns")]
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, multiaddr) in list {
                        println!("mDNS discovered peer: {} at {}", peer_id, multiaddr);
                        let _ = swarm.dial(multiaddr.clone());
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                #[cfg(feature = "mdns")]
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, multiaddr) in list {
                        println!("mDNS peer expired: {} at {}", peer_id, multiaddr);
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => println!(
                        "Got message: '{}' with id: {id} from peer: {peer_id}",
                        String::from_utf8_lossy(&message.data),
                    ),
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    println!("Connected to peer: {peer_id}");
                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                }
                other => {
                    println!("Other swarm event: {other:?}");
                }
            }
        }
    }
}
