#[cfg(feature = "mdns")]
use libp2p::mdns;
use libp2p::{gossipsub, noise, request_response, swarm::NetworkBehaviour, tcp, yamux};
use p2p_app::ChatCodec;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::Duration,
};

#[cfg(not(feature = "tui"))]
use tokio::io::{AsyncBufReadExt as _, BufReader, stdin};

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    request_response: request_response::Behaviour<ChatCodec>,
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
    let mdns = mdns::tokio::Behaviour::new(
        mdns::Config {
            query_interval: Duration::from_secs(1),
            ..Default::default()
        },
        key.public().to_peer_id(),
    )
    .expect("mDNS should be created");

    let request_response = request_response::Behaviour::<p2p_app::ChatCodec>::new(
        vec![(
            libp2p::StreamProtocol::new(p2p_app::DM_PROTOCOL_NAME),
            libp2p::request_response::ProtocolSupport::Full,
        )],
        request_response::Config::default(),
    );

    MyBehaviour {
        gossipsub,
        request_response,
        #[cfg(feature = "mdns")]
        mdns,
    }
}

#[cfg(feature = "tui")]
mod tui {
    use super::MyBehaviour;
    use libp2p::Swarm;
    #[cfg(feature = "mdns")]
    use libp2p::mdns;
    use libp2p::{futures::StreamExt, gossipsub, swarm::SwarmEvent};
    use p2p_app::{
        DirectMessage, get_unsent_messages, load_direct_messages, load_messages, load_peers,
        mark_message_sent, save_message, save_peer,
    };
    use ratatui::crossterm::{
        event::{Event, KeyCode, KeyModifiers, read},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };
    use ratatui::{
        Terminal,
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout},
        style::{Color, Style},
        widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    };
    use std::collections::VecDeque;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const MAX_MESSAGES: usize = 1000;
    const MAX_LOGS: usize = 1000;

    fn format_naive_datetime(time: chrono::NaiveDateTime) -> String {
        let secs = time.and_utc().timestamp();
        let hours = (secs / 3600) % 24;
        let mins = (secs / 60) % 60;
        let s = secs % 60;
        format!("{:02}:{:02}:{:02}", hours, mins, s)
    }

    fn now_timestamp() -> String {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs();
        let hours = (secs / 3600) % 24;
        let mins = (secs / 60) % 60;
        let s = secs % 60;
        format!("{:02}:{:02}:{:02}", hours, mins, s)
    }

    fn format_system_time(time: SystemTime) -> String {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs();
        let hours = (secs / 3600) % 24;
        let mins = (secs / 60) % 60;
        let s = secs % 60;
        format!("{:02}:{:02}:{:02}", hours, mins, s)
    }

    pub async fn run_tui(
        mut swarm: Swarm<MyBehaviour>,
        topic_str: String,
    ) -> color_eyre::Result<()> {
        let is_tty = atty::is(atty::Stream::Stdout);
        if !is_tty {
            return run_noninteractive_mode(swarm).await;
        }

        match Terminal::new(CrosstermBackend::new(std::io::stdout())) {
            Ok(mut terminal) => {
                execute!(std::io::stdout(), EnterAlternateScreen)?;
                enable_raw_mode()?;

                let mut messages: VecDeque<String> = VecDeque::new();
                let mut direct_messages: VecDeque<String> = VecDeque::new();
                let mut logs: VecDeque<String> = VecDeque::new();
                let mut peers: VecDeque<(String, String, String)> = VecDeque::new();
                let mut active_tab = 0;
                let mut selected_peer: Option<String> = None;
                let mut input_buffer = String::new();

                logs.push_back("TUI started".to_string());

                if let Ok(db_messages) = load_messages(&topic_str, MAX_MESSAGES) {
                    for msg in db_messages.iter().rev() {
                        let ts = format_naive_datetime(msg.created_at);
                        let sender = msg
                            .peer_id
                            .as_ref()
                            .map(|p| format!("[{}]", &p[..8.min(p.len())]))
                            .unwrap_or_else(|| "[You]".to_string());
                        messages.push_back(format!("{} {} {}", ts, sender, msg.content));
                    }
                    logs.push_back(format!(
                        "Loaded {} messages from database",
                        db_messages.len()
                    ));
                } else {
                    logs.push_back("Failed to load messages from database".to_string());
                }

                if let Ok(db_peers) = load_peers() {
                    for peer in db_peers.iter() {
                        let first_seen = format_naive_datetime(peer.first_seen);
                        let last_seen = format_naive_datetime(peer.last_seen);
                        peers.push_back((peer.peer_id.to_string(), first_seen, last_seen));
                    }
                    logs.push_back(format!("Loaded {} peers from database", db_peers.len()));
                }

                let tab_titles = vec!["Chat", "Peers", "Direct", "Debug"];

                loop {
                    tokio::select! {
                        biased;

                        event = swarm.select_next_some() => {
                            match event {
                                SwarmEvent::Behaviour(super::MyBehaviourEvent::Gossipsub(
                                    gossipsub::Event::Message {
                                        propagation_source: peer_id,
                                        message,
                                        ..
                                    },
                                )) => {
                                    let peer_id_str = peer_id.to_string();
                                    let content = String::from_utf8_lossy(&message.data).to_string();
                                    let ts = format_system_time(SystemTime::now());
                                    let msg = format!("{} [{}] {}", ts, &peer_id_str[..8], content);
                                    messages.push_back(msg.clone());
                                    if messages.len() > MAX_MESSAGES {
                                        messages.pop_front();
                                    }
                                    if let Err(e) = save_message(&content, Some(&peer_id_str), &topic_str, false, None) {
                                        logs.push_back(format!("Failed to save message: {}", e));
                                    }
                                }
                                SwarmEvent::Behaviour(super::MyBehaviourEvent::RequestResponse(
                                    libp2p::request_response::Event::Message { message, .. },
                                )) => {
                                    match message {
                                        libp2p::request_response::Message::Request { request, channel, .. } => {
                                            let peer_id_str = "unknown".to_string();
                                            let content = request.content.clone();
                                            let ts = format_system_time(SystemTime::now());

                                            if selected_peer.clone() == Some(peer_id_str.clone()) {
                                                let msg = format!("{} [Peer] {}", ts, content);
                                                direct_messages.push_back(msg.clone());
                                                if direct_messages.len() > MAX_MESSAGES {
                                                    direct_messages.pop_front();
                                                }
                                            } else {
                                                logs.push_back(format!("Received DM from unknown: {}", content));
                                            }

                                            if let Err(e) = save_message(&content, Some(&peer_id_str), &topic_str, true, Some(&peer_id_str)) {
                                                logs.push_back(format!("Failed to save DM: {}", e));
                                            }

                                            let response = DirectMessage {
                                                content: "ok".to_string(),
                                                timestamp: SystemTime::now()
                                                    .duration_since(UNIX_EPOCH)
                                                    .map(|d| d.as_secs() as i64)
                                                    .unwrap_or(0),
                                            };
                                            let _ = swarm.behaviour_mut().request_response.send_response(channel, response);
                                        }
                                        libp2p::request_response::Message::Response { request_id, response: _ } => {
                                            let _ = request_id;
                                            logs.push_back("DM sent successfully".to_string());
                                        }
                                    }
                                }
                                #[cfg(feature = "mdns")]
                                SwarmEvent::Behaviour(super::MyBehaviourEvent::Mdns(
                                    mdns::Event::Discovered(list),
                                )) => {
                                    for (peer_id, multiaddr) in list {
                                        let log = format!("mDNS discovered: {} at {}", peer_id, multiaddr);
                                        logs.push_back(log);
                                        let peer_id_str = peer_id.to_string();
                                        peers.push_back((peer_id_str, now_timestamp(), now_timestamp()));
                                        let _ = swarm.dial(multiaddr.clone());
                                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                                    }
                                }
                                #[cfg(feature = "mdns")]
                                SwarmEvent::Behaviour(super::MyBehaviourEvent::Mdns(
                                    mdns::Event::Expired(list),
                                )) => {
                                    for (peer_id, multiaddr) in list {
                                        let log = format!("mDNS expired: {} at {}", peer_id, multiaddr);
                                        logs.push_back(log);
                                        swarm
                                            .behaviour_mut()
                                            .gossipsub
                                            .remove_explicit_peer(&peer_id);
                                    }
                                }
                                SwarmEvent::NewListenAddr { address, .. } => {
                                    let log = format!("Listening on: {}", address);
                                    logs.push_back(log);
                                }
                                SwarmEvent::ConnectionEstablished { peer_id, connection_id, .. } => {
                                    let log = format!("Connected to: {} (conn: {:?})", peer_id, connection_id);
                                    logs.push_back(log);
                                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);

                                    let peer_id_str = peer_id.to_string();
                                    let addresses = vec![peer_id_str.clone()];
                                    match save_peer(&peer_id_str, &addresses) {
                                        Ok(peer) => {
                                            let first_seen = format_naive_datetime(peer.first_seen);
                                            let last_seen = format_naive_datetime(peer.last_seen);
                                            peers.push_back((peer_id_str, first_seen, last_seen));
                                        }
                                        Err(e) => {
                                            peers.push_back((peer_id_str.clone(), now_timestamp(), now_timestamp()));
                                            logs.push_back(format!("Failed to save peer: {}", e));
                                        }
                                    }

                                    if let Ok(unsent) = get_unsent_messages(&topic_str)
                                        && !unsent.is_empty()
                                    {
                                        logs.push_back(format!("Retrying {} unsent messages", unsent.len()));
                                        for msg in unsent {
                                            let topic = gossipsub::IdentTopic::new("test-net");
                                            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, msg.content.as_bytes()) {
                                                logs.push_back(format!("Retry publish failed: {:?}", e));
                                            } else {
                                                let _ = mark_message_sent(msg.id);
                                                let ts = format_system_time(SystemTime::now());
                                                let retry_msg = format!("{} [You] {} (sent)", ts, msg.content);
                                                messages.push_back(retry_msg);
                                            }
                                        }
                                    }
                                }
                                SwarmEvent::ConnectionClosed {
                                    peer_id, cause, connection_id, ..
                                } => {
                                    let log = format!("Disconnected from: {} (conn: {:?}, cause: {:?})", peer_id, connection_id, cause);
                                    logs.push_back(log);
                                }
                                SwarmEvent::Dialing { peer_id: Some(pid), .. } => {
                                    let log = format!("Dialing: {}", pid);
                                    logs.push_back(log);
                                }
                                SwarmEvent::ExpiredListenAddr { address, .. } => {
                                    let log = format!("Expired listen addr: {}", address);
                                    logs.push_back(log);
                                }
                                SwarmEvent::ListenerError { listener_id, error } => {
                                    let log = format!("Listener error: {:?} - {:?}", listener_id, error);
                                    logs.push_back(log);
                                }
                                SwarmEvent::ListenerClosed { listener_id, reason, addresses } => {
                                    let log = format!("Listener closed: {:?} - {:?} ({:?})", listener_id, reason, addresses);
                                    logs.push_back(log);
                                }
                                SwarmEvent::IncomingConnection { connection_id, local_addr, .. } => {
                                    let log = format!("Incoming connection: {:?} from {:?}", connection_id, local_addr);
                                    logs.push_back(log);
                                }
                                SwarmEvent::IncomingConnectionError { connection_id, local_addr, send_back_addr, peer_id, error } => {
                                    let log = format!("Incoming connection error: {:?} from {:?} to {:?} (peer: {:?}): {:?}",
                                        connection_id, local_addr, send_back_addr, peer_id, error);
                                    logs.push_back(log);
                                }
                                _ => {}
                            }

                            if logs.len() > MAX_LOGS {
                                logs.pop_front();
                            }
                        }

                        _ = tokio::time::sleep(Duration::from_millis(16)) => {
                            if let Ok(event) = read()
                                && let Event::Key(key) = event {
                                    match key.code {
                                        KeyCode::Tab => {
                                            if key.modifiers.contains(KeyModifiers::SHIFT) {
                                                active_tab = if active_tab == 0 { 3 } else { active_tab - 1 };
                                            } else {
                                                active_tab = (active_tab + 1) % 4;
                                            }
                                        }
                                        KeyCode::Enter => {
                                            if !input_buffer.is_empty() && active_tab == 0 {
                                                let ts = format_system_time(SystemTime::now());
                                                let msg_str = format!("{} [You] {}", ts, input_buffer);
                                                messages.push_back(msg_str.clone());

                                                let publish_result = swarm.behaviour_mut().gossipsub.publish(
                                                    gossipsub::IdentTopic::new("test-net"),
                                                    input_buffer.as_bytes(),
                                                );

                                                if let Err(e) = publish_result {
                                                    logs.push_back(format!("Publish error: {:?}", e));
                                                }

                                                if let Err(e) = save_message(&input_buffer, None, &topic_str, false, None) {
                                                    logs.push_back(format!("Failed to save message: {}", e));
                                                }

                                                input_buffer.clear();
                                            } else if active_tab == 1 && !peers.is_empty() {
                                                if let Some(first_peer) = peers.front().cloned() {
                                                    let (peer_id, _, _) = first_peer;
                                                    selected_peer = Some(peer_id.clone());
                                                    active_tab = 2;
                                                    direct_messages.clear();
                                                    if let Ok(msgs) = load_direct_messages(&peer_id, MAX_MESSAGES) {
                                                        for msg in msgs {
                                                            let ts = format_naive_datetime(msg.created_at);
                                                            let sender = if msg.peer_id.is_some() { "[You]" } else { "[Peer]" };
                                                            direct_messages.push_back(format!("{} {} {}", ts, sender, msg.content));
                                                        }
                                                    }
                                                }
                                            } else if !input_buffer.is_empty() && active_tab == 2 {
                                                let Some(target) = selected_peer.as_ref() else { continue; };
                                                let ts = format_system_time(SystemTime::now());
                                                let msg_str = format!("{} [You] {}", ts, input_buffer);
                                                direct_messages.push_back(msg_str.clone());

                                                let peer_id: libp2p::PeerId = match target.parse() {
                                                    Ok(pid) => pid,
                                                    Err(e) => {
                                                        logs.push_back(format!("Invalid peer ID: {}", e));
                                                        input_buffer.clear();
                                                        continue;
                                                    }
                                                };

                                                let dm = DirectMessage {
                                                    content: input_buffer.clone(),
                                                    timestamp: SystemTime::now()
                                                        .duration_since(UNIX_EPOCH)
                                                        .map(|d| d.as_secs() as i64)
                                                        .unwrap_or(0),
                                                };

                                                swarm.behaviour_mut().request_response.send_request(&peer_id, dm);

                                                if let Err(e) = save_message(&input_buffer, None, &topic_str, true, Some(target)) {
                                                    logs.push_back(format!("Failed to save DM: {}", e));
                                                }

                                                input_buffer.clear();
                                            }
                                        }
                                        KeyCode::Char(c) => {
                                            input_buffer.push(c);
                                            if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'q' {
                                                execute!(std::io::stdout(), LeaveAlternateScreen).ok();
                                                disable_raw_mode().ok();
                                                return Ok(());
                                            }
                                        }
                                        KeyCode::Backspace => {
                                            input_buffer.pop();
                                        }
                                        KeyCode::Esc => {
                                            execute!(std::io::stdout(), LeaveAlternateScreen).ok();
                                            disable_raw_mode().ok();
                                            return Ok(());
                                        }
                                        _ => {}
                                    }
                                }
                        }
                    }

                    terminal.draw(|f| {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(3),
                                Constraint::Min(0),
                                Constraint::Length(3),
                                Constraint::Length(1),
                            ])
                            .split(f.area());

                        let tabs = Tabs::new(tab_titles.clone())
                            .style(Style::default().fg(Color::Cyan))
                            .select(active_tab);
                        f.render_widget(tabs, chunks[0]);

                        match active_tab {
                            0 => {
                                let chat_items: Vec<ListItem> =
                                    messages.iter().map(|m| ListItem::new(m.clone())).collect();
                                let chat_list = List::new(chat_items)
                                    .block(Block::default().title("Messages").borders(Borders::ALL))
                                    .style(Style::default().fg(Color::White));
                                f.render_widget(chat_list, chunks[1]);
                            }
                            1 => {
                                let peer_items: Vec<ListItem> = peers
                                    .iter()
                                    .map(|(peer_id, first_seen, last_seen)| {
                                        ListItem::new(format!(
                                            "{} | First: {} | Last: {}",
                                            peer_id, first_seen, last_seen
                                        ))
                                    })
                                    .collect();
                                let peer_list = List::new(peer_items)
                                    .block(
                                        Block::default()
                                            .title("Peers - Press Enter to open DM")
                                            .borders(Borders::ALL),
                                    )
                                    .style(Style::default().fg(Color::White));
                                f.render_widget(peer_list, chunks[1]);
                            }
                            2 => {
                                let peer_name = selected_peer
                                    .as_ref()
                                    .map(|p| &p[..8.min(p.len())])
                                    .unwrap_or("None");
                                let dm_items: Vec<ListItem> = direct_messages
                                    .iter()
                                    .map(|m| ListItem::new(m.clone()))
                                    .collect();
                                let dm_list = List::new(dm_items)
                                    .block(
                                        Block::default()
                                            .title(format!("DM: {}", peer_name))
                                            .borders(Borders::ALL),
                                    )
                                    .style(Style::default().fg(Color::White));
                                f.render_widget(dm_list, chunks[1]);
                            }
                            3 => {
                                let log_items: Vec<ListItem> =
                                    logs.iter().map(|l| ListItem::new(l.clone())).collect();
                                let log_list = List::new(log_items)
                                    .block(
                                        Block::default().title("Debug Logs").borders(Borders::ALL),
                                    )
                                    .style(Style::default().fg(Color::White));
                                f.render_widget(log_list, chunks[1]);
                            }
                            _ => {}
                        }

                        let input_display = if active_tab == 0 || active_tab == 2 {
                            format!("> {}", input_buffer)
                        } else {
                            "(Input disabled)".to_string()
                        };
                        let input_line = Paragraph::new(input_display)
                            .style(Style::default().fg(Color::Yellow))
                            .block(Block::default().title("Input").borders(Borders::ALL));
                        f.render_widget(input_line, chunks[2]);

                        let help = Paragraph::new(
                            "Tab: switch | Type message and Enter to send | Ctrl+Q: quit",
                        )
                        .style(Style::default().fg(Color::DarkGray));
                        f.render_widget(help, chunks[3]);
                    })?;
                }
            }
            Err(_e) => run_noninteractive_mode(swarm).await,
        }
    }

    async fn run_noninteractive_mode(mut swarm: Swarm<MyBehaviour>) -> color_eyre::Result<()> {
        println!("Running in non-interactive mode");
        println!("Press Ctrl+C to exit");

        loop {
            tokio::select! {
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            eprintln!("Listening on: {}", address);
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            eprintln!("Connected to: {}", peer_id);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                        SwarmEvent::Behaviour(super::MyBehaviourEvent::Gossipsub(
                            gossipsub::Event::Message { propagation_source, message, .. }
                        )) => {
                            eprintln!(
                                "[{}] {}",
                                &propagation_source.to_string()[..8],
                                String::from_utf8_lossy(&message.data)
                            );
                        }
                        #[cfg(feature = "mdns")]
                        SwarmEvent::Behaviour(super::MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                            for (peer_id, multiaddr) in list {
                                eprintln!("mDNS discovered: {} at {}", peer_id, multiaddr);
                                let addresses = vec![multiaddr.to_string()];
                                if let Err(e) = save_peer(&peer_id.to_string(), &addresses) {
                                    eprintln!("Failed to save peer: {}", e);
                                }
                                let _ = swarm.dial(multiaddr.clone());
                                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                            }
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(3600)) => {}
            }
        }
    }
}

#[cfg(feature = "tui")]
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let mut swarm = {
        let base = libp2p::SwarmBuilder::with_existing_identity(p2p_app::get_libp2p_identity()?)
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

    let topic = gossipsub::IdentTopic::new("test-net");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    #[cfg(feature = "quic")]
    {
        swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?).ok();
    }
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?).ok();

    tui::run_tui(swarm, topic.to_string()).await
}

#[cfg(not(feature = "tui"))]
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
        let base = libp2p::SwarmBuilder::with_existing_identity(p2p_app::get_libp2p_identity()?)
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

    let topic = gossipsub::IdentTopic::new("test-net");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    let mut stdin = BufReader::new(stdin()).lines();

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
                        Err(e) => {
                            println!("Failed to parse multiaddr: {e}");
                        }
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
