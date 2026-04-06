use libp2p::{gossipsub, noise, tcp, yamux};
use p2p_app::{AppBehaviour, build_behaviour, init_logging, p2plog_warn};
use std::collections::VecDeque;
use std::time::Duration;

#[cfg(feature = "tui")]
mod tui {
    use super::AppBehaviour;
    use libp2p::Swarm;
    #[cfg(feature = "mdns")]
    use libp2p::mdns;
    use libp2p::{futures::StreamExt, gossipsub, swarm::SwarmEvent};
    use p2p_app::{
        AppBehaviourEvent as AppEv, DirectMessage, NetworkSize, format_peer_datetime,
        get_database_url, get_network_size, get_unsent_messages, init_logging,
        load_direct_messages, load_listen_ports, load_messages, load_peers, mark_message_sent,
        now_timestamp, p2plog_error, p2plog_info, save_listen_ports, save_message, save_peer,
        save_peer_session,
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
        widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    };
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime};
    use tracing_subscriber::prelude::*;

    const MAX_MESSAGES: usize = 1000;
    const MAX_LOGS: usize = 1000;

    mod tracing_writer {
        use p2p_app::strip_ansi_codes;
        use std::collections::VecDeque;
        use std::sync::{Arc, Mutex};
        use std::time::SystemTime;

        fn format_system_time(time: SystemTime) -> String {
            let local: chrono::DateTime<chrono::Local> = time.into();
            local.format("%H:%M:%S.%3f").to_string()
        }

        #[derive(Clone)]
        pub struct TracingWriter {
            logs: Arc<Mutex<VecDeque<String>>>,
        }

        impl TracingWriter {
            pub fn new(logs: Arc<Mutex<VecDeque<String>>>) -> Self {
                Self { logs }
            }
        }

        impl std::io::Write for TracingWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if let Ok(s) = std::str::from_utf8(buf) {
                    let cleaned = strip_ansi_codes(s);
                    let trimmed = cleaned.trim();
                    if !trimmed.is_empty() {
                        let ts = format_system_time(SystemTime::now());
                        let formatted = format!("[{}] {}", ts, trimmed);
                        if let Ok(mut l) = self.logs.lock() {
                            l.push_back(formatted);
                            if l.len() > 2000 {
                                l.pop_front();
                            }
                        }
                    }
                }
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
    }

    fn format_system_time(time: SystemTime) -> String {
        let local: chrono::DateTime<chrono::Local> = time.into();
        local.format("%H:%M:%S.%3f").to_string()
    }

    fn log_debug(logs: &Mutex<VecDeque<String>>, message: impl Into<String>) {
        let ts = format_system_time(SystemTime::now());
        let formatted = format!("[{}] {}", ts, message.into());
        if let Ok(mut l) = logs.lock() {
            l.push_back(formatted);
        }
    }

    pub async fn run_tui(
        mut swarm: Swarm<AppBehaviour>,
        topic_str: String,
        logs: Arc<Mutex<VecDeque<String>>>,
    ) -> color_eyre::Result<()> {
        let is_tty = atty::is(atty::Stream::Stdout);
        if !is_tty {
            return run_headless_mode(swarm).await;
        }

        match Terminal::new(CrosstermBackend::new(std::io::stdout())) {
            Ok(mut terminal) => {
                execute!(std::io::stdout(), EnterAlternateScreen)?;
                enable_raw_mode()?;

                let mut messages: VecDeque<String> = VecDeque::new();
                let mut direct_messages: VecDeque<String> = VecDeque::new();
                let mut peers: VecDeque<(String, String, String)> = VecDeque::new();
                let mut active_tab = 0;
                let mut selected_peer: Option<String> = None;
                let mut input_buffer = String::new();
                let mut concurrent_peers: usize = 0;
                let mut peer_selection: usize = 0;
                let mut debug_scroll_offset: usize = 0;

                init_logging();
                let logs_for_callback = logs.clone();
                p2p_app::set_tui_log_callback(move |msg| {
                    if let Ok(mut l) = logs_for_callback.lock() {
                        l.push_back(msg);
                        if l.len() > 1000 {
                            l.pop_front();
                        }
                    }
                });

                let logs_for_tracing = logs.clone();
                let trace_layer = tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .without_time()
                    .with_writer(move || {
                        tracing_writer::TracingWriter::new(logs_for_tracing.clone())
                    })
                    .compact()
                    .with_filter(p2p_app::tracing_filter());
                let _ = tracing_subscriber::registry().with(trace_layer).try_init();
                log_debug(&logs, format!("Using database: {}", get_database_url()));
                log_debug(&logs, format!("Our peer ID: {}", swarm.local_peer_id()));

                if let Ok(db_messages) = load_messages(&topic_str, MAX_MESSAGES) {
                    for msg in db_messages.iter().rev() {
                        let ts = format_peer_datetime(msg.created_at);
                        let sender = msg
                            .peer_id
                            .as_ref()
                            .map(|p| format!("[{}]", &p[p.len().saturating_sub(8.min(p.len()))..]))
                            .unwrap_or_else(|| "[You]".to_string());
                        messages.push_back(format!("{} {} {}", ts, sender, msg.content));
                    }
                    log_debug(
                        &logs,
                        format!("Loaded {} messages from database", db_messages.len()),
                    );
                } else {
                    log_debug(&logs, "Failed to load messages from database".to_string());
                }

                if let Ok(db_peers) = load_peers() {
                    let mut peer_indices: Vec<usize> = (0..db_peers.len()).collect();
                    peer_indices
                        .sort_by(|&a, &b| db_peers[b].last_seen.cmp(&db_peers[a].last_seen));
                    for &idx in peer_indices.iter().take(10) {
                        let peer = &db_peers[idx];
                        let first_seen = format_peer_datetime(peer.first_seen);
                        let last_seen = format_peer_datetime(peer.last_seen);
                        peers.push_back((peer.peer_id.to_string(), first_seen, last_seen));
                    }
                    log_debug(
                        &logs,
                        format!(
                            "Loaded {} peers from database (dialing top 10 by last_seen)",
                            db_peers.len()
                        ),
                    );
                    for &idx in peer_indices.iter().take(10) {
                        let peer = &db_peers[idx];
                        let addr_strs: Vec<&str> = peer.addresses.split(',').collect();
                        for addr_str in addr_strs {
                            let trimmed = addr_str.trim();
                            if trimmed.contains("/tcp/") {
                                let addr: libp2p::Multiaddr = match trimmed.parse() {
                                    Ok(a) => a,
                                    Err(e) => {
                                        log_debug(
                                            &logs,
                                            format!(
                                                "Failed to parse peer address '{}': {}",
                                                trimmed, e
                                            ),
                                        );
                                        continue;
                                    }
                                };
                                log_debug(
                                    &logs,
                                    format!("Dialing known peer: {} at {}", peer.peer_id, addr),
                                );
                                swarm.dial(addr).ok();
                            }
                        }
                    }
                } else {
                    log_debug(&logs, "Failed to load peers from database".to_string());
                }

                match get_network_size() {
                    Ok(NetworkSize::Small) => {
                        log_debug(
                            &logs,
                            "Network size: Small (0-3 peers avg) - optimized for low latency",
                        );
                    }
                    Ok(NetworkSize::Medium) => {
                        log_debug(
                            &logs,
                            "Network size: Medium (4-15 peers avg) - balanced settings",
                        );
                    }
                    Ok(NetworkSize::Large) => {
                        log_debug(
                            &logs,
                            "Network size: Large (16+ peers avg) - optimized for bandwidth",
                        );
                    }
                    Err(e) => {
                        log_debug(&logs, format!("Could not determine network size: {}", e));
                    }
                }

                let tab_titles = vec!["Chat", "Peers", "Direct", "Debug"];

                loop {
                    tokio::select! {
                        biased;

                        event = swarm.select_next_some() => {
                            match event {
                                SwarmEvent::Behaviour(AppEv::Gossipsub(
                                    gossipsub::Event::Message {
                                        propagation_source: peer_id,
                                        message,
                                        ..
                                    },
                                )) => {
                                     log_debug(&logs, format!("Gossipsub message from: {}", peer_id));
                                    let peer_id_str = peer_id.to_string();
                                    let content = String::from_utf8_lossy(&message.data).to_string();
                                    let ts = format_system_time(SystemTime::now());
                                    let msg = format!("{} [{}] {}", ts, &peer_id_str[peer_id_str.len().saturating_sub(8)..], content);
                                    messages.push_back(msg.clone());
                                    if messages.len() > MAX_MESSAGES {
                                        messages.pop_front();
                                    }
                                     if let Err(e) = save_message(&content, Some(&peer_id_str), &topic_str, false, None) {
                                         log_debug(&logs, format!("Failed to save message: {}", e));
                                     }
                                }
                                SwarmEvent::Behaviour(AppEv::RequestResponse(
                                    libp2p::request_response::Event::Message { peer, message, .. },
                                )) => {
                                    match message {
                                        libp2p::request_response::Message::Request { request, channel, .. } => {
                                            let peer_id_str = peer.to_string();
                                            let content = request.content.clone();
                                            let ts = format_system_time(SystemTime::now());

                                            if selected_peer.clone() == Some(peer_id_str.clone()) {
                                                let msg = format!("{} [Peer] {}", ts, content);
                                                direct_messages.push_back(msg.clone());
                                                if direct_messages.len() > MAX_MESSAGES {
                                                    direct_messages.pop_front();
                                                }
                                            } else {
                                                 log_debug(&logs, format!("Received DM from {}: {}", &peer_id_str[peer_id_str.len().saturating_sub(8.min(peer_id_str.len()))..], content));
                                            }

                                            if let Err(e) = save_message(&content, Some(&peer_id_str), &topic_str, true, Some(&peer_id_str)) {
                                                 log_debug(&logs, format!("Failed to save DM: {}", e));
                                            }

                                            let response = DirectMessage {
                                                content: "ok".to_string(),
                                                timestamp: chrono::Utc::now().timestamp(),
                                            };
                                            let _ = swarm.behaviour_mut().request_response.send_response(channel, response);
                                        }
                                        libp2p::request_response::Message::Response { request_id, response } => {
                                            let _ = request_id;
                                             log_debug(&logs, format!("DM response received: {}", response.content));
                                        }
                                    }
                                }
                                #[cfg(feature = "mdns")]
                                SwarmEvent::Behaviour(AppEv::Mdns(
                                    mdns::Event::Discovered(list),
                                )) => {
                                    for (peer_id, multiaddr) in list {
                                        let peer_id_str = peer_id.to_string();
                                        let addresses = vec![multiaddr.to_string()];
                                        match save_peer(&peer_id_str, &addresses) {
                                            Ok(peer) => {
                                                let first_seen = format_peer_datetime(peer.first_seen);
                                                let last_seen = format_peer_datetime(peer.last_seen);
                                                if !peers.iter().any(|(id, _, _)| id == &peer_id_str) {
                                                    peers.push_back((peer_id_str.clone(), first_seen, last_seen));
                                                }
                                            }
                                            Err(e) => {
                                                if !peers.iter().any(|(id, _, _)| id == &peer_id_str) {
                                                    peers.push_back((peer_id_str.clone(), now_timestamp(), now_timestamp()));
                                                }
                                                log_debug(&logs, format!("Failed to save peer: {}", e));
                                            }
                                        }
                                        swarm.dial(multiaddr.clone()).ok();
                                        let _ = swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                                    }
                                }
                                #[cfg(feature = "mdns")]
                                SwarmEvent::Behaviour(AppEv::Mdns(
                                    mdns::Event::Expired(list),
                                    )) => {
                                    for (peer_id, multiaddr) in list {
                                        log_debug(&logs, format!("mDNS expired: {} at {}", peer_id, multiaddr));
                                        swarm
                                            .behaviour_mut()
                                            .gossipsub
                                            .remove_explicit_peer(&peer_id);
                                    }
                                }
                                SwarmEvent::NewListenAddr { address, .. } => {
                                    log_debug(&logs, format!("Listening on: {}", address));
                                    if let Some(port) = address
                                        .iter()
                                        .find_map(|p| match p {
                                            libp2p::multiaddr::Protocol::Tcp(port) => Some(port as i32),
                                            _ => None,
                                        })
                                    {
                                        let _ = save_listen_ports(Some(port), None);
                                    }
                                    #[cfg(feature = "quic")]
                                    if let Some(port) = address
                                        .iter()
                                        .find_map(|p| match p {
                                            libp2p::multiaddr::Protocol::Udp(port) => Some(port as i32),
                                            _ => None,
                                        })
                                    {
                                        let (tcp, _) = load_listen_ports().unwrap_or((None, None));
                                        let _ = save_listen_ports(tcp, Some(port));
                                    }
                                }
                                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                                    concurrent_peers += 1;
                                    log_debug(&logs, format!("Concurrent peers: {}", concurrent_peers));
                                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);

                                    let peer_id_str = peer_id.to_string();
                                    let addresses = vec![peer_id_str.clone()];
                                    match save_peer(&peer_id_str, &addresses) {
                                        Ok(peer) => {
                                            let first_seen = format_peer_datetime(peer.first_seen);
                                            let last_seen = format_peer_datetime(peer.last_seen);
                                            if !peers.iter().any(|(id, _, _)| id == &peer_id_str) {
                                                peers.push_back((peer_id_str, first_seen, last_seen));
                                            }
                                        }
                                        Err(e) => {
                                            if !peers.iter().any(|(id, _, _)| id == &peer_id_str) {
                                                peers.push_back((peer_id_str.clone(), now_timestamp(), now_timestamp()));
                                            }
                                            log_debug(&logs, format!("Failed to save peer: {}", e));
                                        }
                                    }

                                    if let Ok(unsent) = get_unsent_messages(&topic_str)
                                        && !unsent.is_empty()
                                    {
                                        log_debug(&logs, format!("Retrying {} unsent messages", unsent.len()));
                                        for msg in unsent {
                                            let topic = gossipsub::IdentTopic::new("test-net");
                                            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, msg.content.as_bytes()) {
                                                log_debug(&logs, format!("Retry publish failed: {:?}", e));
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
                                    peer_id, cause: _, ..
                                } => {
                                    if concurrent_peers > 0 {
                                        concurrent_peers -= 1;
                                    }
                                    log_debug(&logs, format!("Concurrent peers: {} (disconnected: {})", concurrent_peers, &peer_id.to_string()[peer_id.to_string().len().saturating_sub(8)..]));
                                    if let Err(e) = save_peer_session(concurrent_peers as i32) {
                                        log_debug(&logs, format!("Failed to save peer session: {}", e));
                                    }
                                }
                                SwarmEvent::Dialing { peer_id: Some(_pid), .. } => {
                                }
                                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                                    log_debug(&logs, format!("Dial failed: peer={:?}, error={}", peer_id, error));
                                }
                                SwarmEvent::ExpiredListenAddr { address, .. } => {
                                    log_debug(&logs, format!("Expired listen addr: {}", address));
                                }
                                SwarmEvent::ListenerError { listener_id, error } => {
                                    log_debug(&logs, format!("Listener {:?} error: {}", listener_id, error));
                                }
                                SwarmEvent::ListenerClosed { listener_id, reason, addresses } => {
                                    log_debug(&logs, format!("Listener {:?} closed: {:?} ({:?})", listener_id, reason, addresses));
                                }
                                SwarmEvent::IncomingConnection { .. } => {}
                                SwarmEvent::IncomingConnectionError { connection_id, local_addr, send_back_addr, error, .. } => {
                                    let err_str = format!("{}", error);
                                    if err_str.contains("ConnectionClosed") || err_str.contains("TimedOut") {
                                    } else {
                                        log_debug(&logs, format!("Connection {:?} error: {} from {:?} to {:?}",
                                            connection_id, error, local_addr, send_back_addr));
                                    }
                                }
                                _ => {}
                            }

                            if let Ok(l) = logs.lock() {
                                if l.len() > MAX_LOGS {
                                    drop(l);
                                    if let Ok(mut l) = logs.lock() {
                                        l.pop_front();
                                    }
                                }
                            }
                        }

                        _ = tokio::time::sleep(Duration::from_millis(16)) => {
                            if let Ok(event) = read()
                                && let Event::Key(key) = event {
                                    match key.code {
                                        KeyCode::Tab | KeyCode::BackTab => {
                                            if key.code == KeyCode::BackTab {
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

                                                 let topic = gossipsub::IdentTopic::new("test-net");
                                                 log_debug(&logs, format!("Publishing to gossipsub topic: {}", topic));
                                                 let publish_result = swarm.behaviour_mut().gossipsub.publish(
                                                     topic,
                                                     input_buffer.as_bytes(),
                                                 );

                                                 if let Err(e) = publish_result {
                                                     log_debug(&logs, format!("Publish error: {:?}", e));
                                                 } else {
                                                     log_debug(&logs, "Message published successfully".to_string());
                                                 }

                                                 if let Err(e) = save_message(&input_buffer, None, &topic_str, false, None) {
                                                     log_debug(&logs, format!("Failed to save message: {}", e));
                                                 }

                                                 input_buffer.clear();
                                             } else if active_tab == 1 && !peers.is_empty() {
                                                 let idx = peer_selection.min(peers.len() - 1);
                                                 if let Some(peer) = peers.iter().nth(idx).cloned() {
                                                     let (peer_id, _, _) = peer;
                                                     selected_peer = Some(peer_id.clone());
                                                     active_tab = 2;
                                                     direct_messages.clear();
                                                     if let Ok(msgs) = load_direct_messages(&peer_id, MAX_MESSAGES) {
                                                         for msg in msgs {
                                                             let ts = format_peer_datetime(msg.created_at);
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
                                                log_debug(&logs, format!("Sending DM to {}", target));

                                                let peer_id: libp2p::PeerId = match target.parse() {
                                                    Ok(pid) => pid,
                                                    Err(e) => {
                                                        log_debug(&logs, format!("Invalid peer ID: {}", e));
                                                        input_buffer.clear();
                                                        continue;
                                                    }
                                                };

                                                let dm = DirectMessage {
                                                    content: input_buffer.clone(),
                                                    timestamp: chrono::Utc::now().timestamp(),
                                                };

                                                swarm.behaviour_mut().request_response.send_request(&peer_id, dm);
                                                log_debug(&logs, format!("DM request sent to {}", target));

                                                if let Err(e) = save_message(&input_buffer, None, &topic_str, true, Some(&peer_id.to_string())) {
                                                    log_debug(&logs, format!("Failed to save DM: {}", e));
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
                                         KeyCode::Up => {
                                             if active_tab == 1 && !peers.is_empty() {
                                                 peer_selection = peer_selection.saturating_sub(1);
                                             } else if active_tab == 3 {
                                                 debug_scroll_offset = debug_scroll_offset.saturating_sub(1);
                                             }
                                         }
                                         KeyCode::Down => {
                                             if active_tab == 1 && !peers.is_empty() {
                                                 peer_selection = (peer_selection + 1).min(peers.len() - 1);
                                             } else if active_tab == 3 {
                                                 if let Ok(l) = logs.lock() {
                                                     debug_scroll_offset = (debug_scroll_offset + 1).min(l.len().saturating_sub(1));
                                                 }
                                             }
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
                                    .enumerate()
                                    .map(|(i, (peer_id, first_seen, last_seen))| {
                                        let prefix =
                                            if i == peer_selection { ">> " } else { "   " };
                                        ListItem::new(format!(
                                            "{}{} | First: {} | Last: {}",
                                            prefix, peer_id, first_seen, last_seen
                                        ))
                                    })
                                    .collect();
                                let peer_list = List::new(peer_items)
                                    .block(
                                        Block::default()
                                            .title("Peers - Up/Down to select, Enter to open DM")
                                            .borders(Borders::ALL),
                                    )
                                    .style(Style::default().fg(Color::White));
                                f.render_widget(peer_list, chunks[1]);
                            }
                            2 => {
                                let peer_name = selected_peer
                                    .as_ref()
                                    .map(|p| &p[p.len().saturating_sub(8.min(p.len()))..])
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
                                let log_vec = logs.lock().unwrap().clone();
                                let total = log_vec.len();
                                let debug_title =
                                    format!("Debug Logs [{}/{}]", debug_scroll_offset + 1, total);
                                let visible_logs: Vec<String> = log_vec
                                    .iter()
                                    .skip(debug_scroll_offset)
                                    .take(50)
                                    .cloned()
                                    .collect();
                                let log_text = visible_logs.join("\n");
                                let log_paragraph = Paragraph::new(log_text)
                                    .block(
                                        Block::default().title(debug_title).borders(Borders::ALL),
                                    )
                                    .style(Style::default().fg(Color::White))
                                    .wrap(Wrap::default());
                                f.render_widget(log_paragraph, chunks[1]);
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
            Err(_e) => run_headless_mode(swarm).await,
        }
    }

    async fn run_headless_mode(mut swarm: Swarm<AppBehaviour>) -> color_eyre::Result<()> {
        init_logging();
        p2plog_info("Starting non-interactive mode".to_string());
        p2plog_info(format!("Our peer ID: {}", swarm.local_peer_id()));
        p2plog_info("Press Ctrl+C to exit".to_string());

        let mut concurrent_peers: usize = 0;

        loop {
            tokio::select! {
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            p2plog_info(format!("Listening on: {}", address));
                            if let Some(port) = address
                                .iter()
                                .find_map(|p| match p {
                                    libp2p::multiaddr::Protocol::Tcp(port) => Some(port as i32),
                                    _ => None,
                                })
                            {
                                let _ = save_listen_ports(Some(port), None);
                            }
                            #[cfg(feature = "quic")]
                            if let Some(port) = address
                                .iter()
                                .find_map(|p| match p {
                                    libp2p::multiaddr::Protocol::Udp(port) => Some(port as i32),
                                    _ => None,
                                })
                            {
                                let (tcp, _) = load_listen_ports().unwrap_or((None, None));
                                let _ = save_listen_ports(tcp, Some(port));
                            }
                        }
                        SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                            p2plog_info(format!("Dial failed: peer={:?}, error={}", peer_id, error));
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            concurrent_peers += 1;
                            p2plog_info(format!("Connected to: {} (peers: {})", peer_id, concurrent_peers));
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            if concurrent_peers > 0 {
                                concurrent_peers -= 1;
                            }
                            p2plog_info(format!("Disconnected from: {} (peers: {})", peer_id, concurrent_peers));
                            if let Err(e) = save_peer_session(concurrent_peers as i32) {
                                p2plog_error(format!("Failed to save peer session: {}", e));
                            }
                        }
                        SwarmEvent::Dialing { peer_id, .. } => {
                            p2plog_info(format!("Dialing: {:?}", peer_id));
                        }
                        SwarmEvent::Behaviour(AppEv::Gossipsub(
                            gossipsub::Event::Message { propagation_source, message, .. }
                        )) => {
                            let ps = propagation_source.to_string();
                            let suffix = &ps[ps.len().saturating_sub(8)..];
                            p2plog_info(format!(
                                "[{}] {}",
                                suffix,
                                String::from_utf8_lossy(&message.data)
                            ));
                        }
                        #[cfg(feature = "mdns")]
                        SwarmEvent::Behaviour(AppEv::Mdns(mdns::Event::Discovered(list))) => {
                            for (peer_id, multiaddr) in list {
                                p2plog_info(format!("mDNS discovered: {} at {}", peer_id, multiaddr));
                                let addresses = vec![multiaddr.to_string()];
                                if let Err(e) = save_peer(&peer_id.to_string(), &addresses) {
                                    p2plog_error(format!("Failed to save peer: {}", e));
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
    init_logging();

    // Set up TUI logging callback early to capture all logs
    let logs = std::sync::Arc::new(std::sync::Mutex::new(VecDeque::new()));
    let logs_callback = logs.clone();
    p2p_app::set_tui_log_callback(move |msg| {
        if let Ok(mut l) = logs_callback.lock() {
            l.push_back(msg);
            if l.len() > 1000 {
                l.pop_front();
            }
        }
    });

    let topic = gossipsub::IdentTopic::new("test-net");

    let network_size = match p2p_app::get_network_size() {
        Ok(size) => {
            eprintln!("Network size detected: {:?}", size);
            size
        }
        Err(e) => {
            eprintln!(
                "Could not determine network size, defaulting to Small: {}",
                e
            );
            p2p_app::NetworkSize::Small
        }
    };

    let mut swarm = {
        let base = libp2p::SwarmBuilder::with_existing_identity(p2p_app::get_libp2p_identity()?)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )?;

        #[cfg(feature = "quic")]
        let swarm = base
            .with_quic()
            .with_behaviour(|key| Ok(build_behaviour(key, network_size)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        #[cfg(not(feature = "quic"))]
        let swarm = base
            .with_behaviour(|key| Ok(build_behaviour(key, network_size)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        swarm
    };

    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    let (last_tcp_port, last_quic_port) = p2p_app::load_listen_ports().unwrap_or((None, None));

    #[cfg(feature = "quic")]
    {
        let quic_addr = if let Some(port) = last_quic_port {
            format!("/ip4/0.0.0.0/udp/{}/quic-v1", port)
        } else {
            "/ip4/0.0.0.0/udp/0/quic-v1".to_string()
        };
        match quic_addr.parse::<libp2p::Multiaddr>() {
            Ok(addr) => {
                if swarm.listen_on(addr.clone()).is_err() {
                    p2plog_warn(format!(
                        "Failed to listen on last QUIC port {}, trying port 0",
                        addr
                    ));
                    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?).ok();
                }
            }
            Err(e) => {
                p2plog_warn(format!("Failed to parse QUIC address: {}", e));
                swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?).ok();
            }
        }
    }

    let tcp_addr = if let Some(port) = last_tcp_port {
        format!("/ip4/0.0.0.0/tcp/{}", port)
    } else {
        "/ip4/0.0.0.0/tcp/0".to_string()
    };
    match tcp_addr.parse::<libp2p::Multiaddr>() {
        Ok(addr) => {
            if swarm.listen_on(addr.clone()).is_err() {
                p2plog_warn(format!(
                    "Failed to listen on last TCP port {}, trying port 0",
                    addr
                ));
                swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?).ok();
            }
        }
        Err(e) => {
            p2plog_warn(format!("Failed to parse TCP address: {}", e));
            swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?).ok();
        }
    }

    // Clone logs for the TUI function
    let tui_logs = logs.clone();
    tui::run_tui(swarm, topic.to_string(), tui_logs).await
}

#[cfg(not(feature = "tui"))]
mod headless {
    use super::AppBehaviour;
    use libp2p::{
        Multiaddr, futures::StreamExt, gossipsub, mdns, noise, swarm::SwarmEvent, tcp, yamux,
    };
    use p2p_app::{
        AppBehaviourEvent, NetworkSize, build_behaviour, init_logging, load_listen_ports,
        p2plog_debug, p2plog_error, p2plog_info, p2plog_warn, save_listen_ports, save_peer,
    };
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt as _, BufReader, stdin};

    pub async fn run() -> color_eyre::Result<()> {
        color_eyre::install()?;
        init_logging();
        p2plog_info("Starting non-interactive mode".to_string());

        // libp2p uses the tracing library which helps to understand complex async flows
        #[cfg(feature = "tracing")]
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init()
            .map_err(|e| {
                p2plog_error(format!("failed to init tracing subscriber: {e}"));
            })
            .ok();

        let mut swarm = {
            let base =
                libp2p::SwarmBuilder::with_existing_identity(p2p_app::get_libp2p_identity()?)
                    .with_tokio()
                    .with_tcp(
                        tcp::Config::default().nodelay(true),
                        noise::Config::new,
                        yamux::Config::default,
                    )?;

            #[cfg(feature = "quic")]
            let swarm = base
                .with_quic()
                .with_behaviour(|key| Ok(build_behaviour(key, p2p_app::NetworkSize::Small)))?
                .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
                .build();

            #[cfg(not(feature = "quic"))]
            let swarm = base
                .with_behaviour(|key| Ok(build_behaviour(key, p2p_app::NetworkSize::Small)))?
                .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
                .build();

            swarm
        };

        let topic = gossipsub::IdentTopic::new("test-net");
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

        let mut stdin = BufReader::new(stdin()).lines();

        let (last_tcp_port, last_quic_port) = load_listen_ports().unwrap_or((None, None));

        #[cfg(feature = "quic")]
        {
            let quic_addr = if let Some(port) = last_quic_port {
                format!("/ip4/0.0.0.0/udp/{}/quic-v1", port)
            } else {
                "/ip4/0.0.0.0/udp/0/quic-v1".to_string()
            };
            match quic_addr.parse::<Multiaddr>() {
                Ok(addr) => {
                    if swarm.listen_on(addr.clone()).is_ok() {
                        p2plog_info(format!("Listening on QUIC: {}", addr));
                    } else {
                        p2plog_warn(format!("Failed to listen on last QUIC port, trying port 0"));
                        swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?).ok();
                    }
                }
                Err(e) => {
                    p2plog_warn(format!("Failed to parse QUIC address: {}", e));
                    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?).ok();
                }
            }
        }

        let tcp_addr = if let Some(port) = last_tcp_port {
            format!("/ip4/0.0.0.0/tcp/{}", port)
        } else {
            "/ip4/0.0.0.0/tcp/0".to_string()
        };
        match tcp_addr.parse::<Multiaddr>() {
            Ok(addr) => {
                if swarm.listen_on(addr.clone()).is_ok() {
                    p2plog_info(format!("Listening on TCP: {}", addr));
                } else {
                    p2plog_warn(format!("Failed to listen on last TCP port, trying port 0"));
                    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?).ok();
                }
            }
            Err(e) => {
                p2plog_warn(format!("Failed to parse TCP address: {}", e));
                swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?).ok();
            }
        }

        p2plog_info(
            "Enter messages via STDIN and they will be sent to connected peers using Gossipsub"
                .to_string(),
        );
        p2plog_info("Or connect to another peer manually: /connect <multiaddr>".to_string());

        loop {
            tokio::select! {
                Ok(Some(line)) = stdin.next_line() => {
                    let line = line.trim().to_string();
                    if line.starts_with("/connect ") {
                        let addr = line.trim_start_matches("/connect ");
                        match addr.parse::<Multiaddr>() {
                            Ok(multiaddr) => {
                                p2plog_info(format!("Dialing {multiaddr}"));
                                swarm.dial(multiaddr).map_err(|e| p2plog_error(format!("Dial error: {e:?}"))).ok();
                            }
                            Err(e) => {
                                p2plog_error(format!("Failed to parse multiaddr: {e}"));
                            }
                        }
                    } else if line.is_empty() {
                        continue;
                    } else {
                        swarm
                            .behaviour_mut()
                            .gossipsub
                            .publish(topic.clone(), line.as_bytes())
                            .map_err(|e| p2plog_error(format!("Publish error: {e:?}")))
                            .ok();
                    }
                }
                event = swarm.select_next_some() => match event {
                    #[cfg(feature = "mdns")]
                    SwarmEvent::Behaviour(AppBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                        for (peer_id, multiaddr) in list {
                            p2plog_info(format!("mDNS discovered peer: {} at {}", peer_id, multiaddr));
                            let addresses = vec![multiaddr.to_string()];
                            if let Err(e) = save_peer(&peer_id.to_string(), &addresses) {
                                p2plog_error(format!("Failed to save peer: {}", e));
                            }
                            let _ = swarm.dial(multiaddr.clone());
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                    },
                    #[cfg(feature = "mdns")]
                    SwarmEvent::Behaviour(AppBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                        for (peer_id, multiaddr) in list {
                            p2plog_info(format!("mDNS peer expired: {} at {}", peer_id, multiaddr));
                            swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                        }
                    },
                    SwarmEvent::Behaviour(AppBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    })) => p2plog_info(format!(
                        "Got message: '{}' with id: {id} from peer: {peer_id}",
                        String::from_utf8_lossy(&message.data),
                    )),
                    SwarmEvent::NewListenAddr { address, .. } => {
                        p2plog_info(format!("Local node is listening on {address}"));
                        if let Some(port) = address
                            .iter()
                            .find_map(|p| match p {
                                libp2p::multiaddr::Protocol::Tcp(port) => Some(port as i32),
                                _ => None,
                            })
                        {
                            let _ = save_listen_ports(Some(port), None);
                        }
                        #[cfg(feature = "quic")]
                        if let Some(port) = address
                            .iter()
                            .find_map(|p| match p {
                                libp2p::multiaddr::Protocol::Udp(port) => Some(port as i32),
                                _ => None,
                            })
                        {
                            let (tcp, _) = load_listen_ports().unwrap_or((None, None));
                            let _ = save_listen_ports(tcp, Some(port));
                        }
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                        p2plog_error(format!("Dial failed: peer={:?}, error={}", peer_id, error));
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        p2plog_info(format!("Connected to peer: {peer_id}"));
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                    other => {
                        p2plog_debug(format!("Other swarm event: {other:?}"));
                    }
                }
            }
        }
    }
}

#[cfg(not(feature = "tui"))]
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    headless::run().await
}
