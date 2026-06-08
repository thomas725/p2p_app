#[cfg(feature = "dioxus-desktop")]
mod dioxus {
    use dioxus_desktop::tao;
    use dioxus_desktop::{Config, WindowBuilder};
    use libp2p::gossipsub;
    use std::collections::{HashMap, VecDeque};

    type Messages = VecDeque<(String, Option<String>)>;
    type MessageIds = VecDeque<Option<String>>;
    type SentAtByMsgId = HashMap<String, f64>;

    fn format_messages(
        topic_str: &str,
        max_messages: usize,
        local_nicknames: &HashMap<String, String>,
        received_nicknames: &HashMap<String, String>,
        own_nickname: &str,
    ) -> (Messages, MessageIds, SentAtByMsgId) {
        let mut messages = VecDeque::new();
        let mut message_ids = VecDeque::new();
        let mut sent_at_by_msg_id = HashMap::new();
        if let Ok(db_messages) = p2p_app::load_messages(topic_str, max_messages) {
            for msg in db_messages.iter().rev() {
                let ts = p2p_app::format_peer_datetime(msg.created_at);
                let sender = if msg.peer_id.is_none() {
                    msg.sender_nickname
                        .as_ref()
                        .map(|n| format!("[{}]", n))
                        .unwrap_or_else(|| format!("[{}]", own_nickname))
                } else {
                    msg.sender_nickname
                        .as_ref()
                        .map(|n| format!("[{}]", n))
                        .unwrap_or_else(|| {
                            let p = msg.peer_id.as_ref().unwrap();
                            let display =
                                p2p_app::peer_display_name(p, local_nicknames, received_nicknames);
                            format!("[{}]", display)
                        })
                };
                messages.push_back((
                    format!("{} {} {}", ts, sender, msg.content),
                    msg.peer_id.clone(),
                ));
                message_ids.push_back(msg.msg_id.clone());
                if let Some(ref msg_id) = msg.msg_id
                    && let Some(sent_at) = msg.sent_at
                {
                    sent_at_by_msg_id.insert(msg_id.clone(), sent_at);
                }
            }
        }
        (messages, message_ids, sent_at_by_msg_id)
    }

    pub fn main() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        rt.block_on(async {
            p2p_app::logging::init_logging();
            p2p_app::logging::set_tui_callback(|msg| eprintln!("{}", msg));
            let _db = p2p_app::init_database().expect("Failed to init database");

            let network_size = match p2p_app::get_network_size() {
                Ok(size) => {
                    p2p_app::p2plog_info(format!("Network size: {:?}", size));
                    size
                }
                Err(e) => {
                    p2p_app::p2plog_info(format!("Defaulting to Small: {}", e));
                    p2p_app::NetworkSize::Small
                }
            };

            let mut swarm = p2p_app::build_swarm(network_size).expect("Failed to build swarm");

            let _ = swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap());
            let _ = swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&gossipsub::IdentTopic::new("test-net"));

            let (swarm_handler, swarm_event_rx, swarm_cmd_tx) =
                p2p_app::spawn_swarm_handler(swarm, "test-net".to_string());

            p2p_app::dioxus_app::SWARM_CMD_TX
                .set(std::sync::Mutex::new(swarm_cmd_tx))
                .ok();

            let own_nickname =
                p2p_app::ensure_self_nickname().unwrap_or_else(|_| "Anonymous".to_string());

            let (local_nicknames, received_nicknames, self_nicknames_for_peers) =
                if let Ok(db_peers) = p2p_app::load_peers() {
                    let mut local = HashMap::new();
                    let mut received = HashMap::new();
                    let mut self_for_peer = HashMap::new();
                    for p in db_peers {
                        if let Some(n) = p.peer_local_nickname {
                            local.insert(p.peer_id.clone(), n);
                        }
                        if let Some(n) = p.received_nickname {
                            received.insert(p.peer_id.clone(), n);
                        }
                        if let Some(n) = p.self_nickname_for_peer {
                            self_for_peer.insert(p.peer_id.clone(), n);
                        }
                    }
                    (local, received, self_for_peer)
                } else {
                    (HashMap::new(), HashMap::new(), HashMap::new())
                };

            let (initial_messages, initial_message_ids, loaded_sent_at) = format_messages(
                "test-net",
                1000,
                &local_nicknames,
                &received_nicknames,
                &own_nickname,
            );

            let mut initial_peers = if let Ok(db_peers) = p2p_app::load_known_peers() {
                let mut peers = VecDeque::new();
                let mut seen = std::collections::HashSet::new();
                for p in &db_peers {
                    if !seen.insert(p.peer_id.clone()) {
                        continue;
                    }
                    peers.push_back((
                        p.peer_id.clone(),
                        p2p_app::format_peer_datetime(p.first_seen),
                        p2p_app::format_peer_datetime(p.last_seen),
                    ));
                }
                peers
            } else {
                VecDeque::new()
            };

            let mut pv: Vec<_> = initial_peers.drain(..).collect();
            pv.sort_by(|a, b| b.2.cmp(&a.2));
            initial_peers = pv.into();

            let mut broadcast_receipts: HashMap<String, HashMap<String, f64>> = HashMap::new();
            let mut dm_receipts: HashMap<String, (String, f64)> = HashMap::new();
            if let Ok(receipts) = p2p_app::load_receipts() {
                for r in receipts {
                    if r.kind == 0 {
                        broadcast_receipts
                            .entry(r.msg_id.clone())
                            .or_default()
                            .insert(r.peer_id.clone(), r.confirmed_at);
                    } else {
                        dm_receipts.insert(r.msg_id.clone(), (r.peer_id.clone(), r.confirmed_at));
                    }
                }
            }

            let local_peer_id = p2p_app::get_local_peer_id()
                .map(|id| id.to_string())
                .unwrap_or_else(|_| "unknown".to_string());

            let init_data = p2p_app::dioxus_app::InitData {
                own_nickname,
                local_peer_id,
                topic_str: "test-net".to_string(),
                local_nicknames,
                received_nicknames,
                self_nicknames_for_peers,
                messages: initial_messages,
                message_ids: initial_message_ids,
                sent_at: loaded_sent_at,
                peers: initial_peers,
                broadcast_receipts,
                dm_receipts,
            };
            p2p_app::dioxus_app::INIT_DATA
                .set(std::sync::Mutex::new(Some(init_data)))
                .ok();

            tokio::spawn(async move {
                let _ = swarm_handler.await;
            });

            tokio::spawn(async move {
                let mut rx = swarm_event_rx;
                while let Some(event) = rx.recv().await {
                    if let Some(tx) = p2p_app::dioxus_app::SWARM_EVENT_TX.get() {
                        let _ = tx.unbounded_send(event);
                    }
                }
            });
        });

        let _guard = rt.enter();

        let config = Config::new()
            .with_window(
                WindowBuilder::new()
                    .with_title("P2P Chat Desktop")
                    .with_inner_size(tao::dpi::LogicalSize::new(900.0, 700.0)),
            )
            .with_custom_head(
                r#"<style>html,body{margin:0;padding:0;height:100%;overflow:hidden;}</style>"#
                    .to_string(),
            );

        dioxus_desktop::launch::launch(
            p2p_app::dioxus_app::App as fn() -> dioxus::prelude::Element,
            vec![],
            vec![Box::new(config)],
        );
    }
}

#[cfg(not(feature = "dioxus-desktop"))]
fn main() {
    eprintln!("This binary requires the 'dioxus-desktop' feature.");
    eprintln!("Run with: cargo run --bin p2p_chat_dioxus --features dioxus-desktop");
}

#[cfg(feature = "dioxus-desktop")]
fn main() {
    dioxus::main();
}
