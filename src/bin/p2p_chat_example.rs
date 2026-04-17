use libp2p::{gossipsub, noise, tcp, yamux};
use p2p_app::{AppBehaviour, build_behaviour, init_logging};
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
        AppBehaviourEvent as AppEv, DirectMessage, DynamicTabs, NetworkSize, TabContent,
        format_peer_datetime, get_database_url, get_network_size, get_unsent_messages,
        init_logging, load_direct_messages, load_listen_ports, load_messages, load_peers,
        mark_message_sent, now_timestamp, p2plog_error, p2plog_info, save_listen_ports,
        save_message, save_peer, save_peer_session,
    };
    use ratatui::crossterm::{
        event::{
            Event, KeyCode, KeyModifiers, PopKeyboardEnhancementFlags,
            PushKeyboardEnhancementFlags, poll, read,
        },
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
    use ratatui_textarea::{Input, TextArea};
    use std::collections::{HashMap, VecDeque};
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

    fn format_latency(sent_at: Option<f64>, received_at: SystemTime) -> String {
        if let Some(sent) = sent_at {
            let recv = received_at
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time is valid")
                .as_secs_f64();
            let diff = recv - sent;
            if diff.abs() < 10.0 {
                format!("[{:.3}s]", diff)
            } else {
                let sent_str = format_system_time(
                    std::time::UNIX_EPOCH + std::time::Duration::from_secs_f64(sent),
                );
                let recv_str = format_system_time(received_at);
                format!("[sent:{} recv:{}]", sent_str, recv_str)
            }
        } else {
            String::new()
        }
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
                execute!(
                    std::io::stdout(),
                    PushKeyboardEnhancementFlags(
                        crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    )
                )?;
                execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
                let mut mouse_capture = true;

                let mut messages: VecDeque<(String, Option<String>)> = VecDeque::new();
                let mut dm_messages: HashMap<String, VecDeque<String>> = HashMap::new();
                let mut peers: VecDeque<(String, String, String)> = VecDeque::new();
                let mut dynamic_tabs = DynamicTabs::new();
                let mut active_tab = 0;
                let mut dm_inputs: HashMap<String, TextArea> = HashMap::new();
                let mut chat_input = TextArea::default();
                chat_input.set_line_number_style(Style::default());
                chat_input.set_cursor_line_style(Style::default());
                let mut concurrent_peers: usize = 0;
                let mut peer_selection: usize = 0;
                let mut debug_scroll_offset: usize = 0;
                let mut debug_auto_scroll: bool = true;
                let mut chat_scroll_offset: usize = 0;
                let mut chat_auto_scroll: bool = true;

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
                        messages.push_back((
                            format!("{} {} {}", ts, sender, msg.content),
                            msg.peer_id.clone(),
                        ));
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

                let mut unread_broadcasts: u32 = 0;
                let mut unread_dms: HashMap<String, u32> = HashMap::new();

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
                                    let peer_id_str = peer_id.to_string();
                                    let raw = String::from_utf8_lossy(&message.data).to_string();
                                    let now = SystemTime::now();
                                    let ts = format_system_time(now);

                                    let (content, sent_at) = if let Ok(bcast) = serde_json::from_str::<p2p_app::BroadcastMessage>(&raw) {
                                        (bcast.content, bcast.sent_at)
                                    } else {
                                        (raw, None)
                                    };

                                    let latency = format_latency(sent_at, now);
                                    let msg = format!("{} {} [{}] {}", ts, latency, &peer_id_str[peer_id_str.len().saturating_sub(8.min(peer_id_str.len()))..], content);
                                    messages.push_back((msg.clone(), Some(peer_id_str.clone())));
                                    if messages.len() > MAX_MESSAGES {
                                        messages.pop_front();
                                    }
                                    if active_tab != 0 {
                                        unread_broadcasts += 1;
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
                                            let now = SystemTime::now();
                                            let ts = format_system_time(now);
                                            let latency = format_latency(request.sent_at, now);

                                            let current_peer = match dynamic_tabs.tab_index_to_content(active_tab) {
                                                TabContent::Direct(id) => Some(id),
                                                _ => None,
                                            };
                                            let is_current_dm = current_peer.as_ref() == Some(&peer_id_str);

                                            let dm_msgs = dm_messages.entry(peer_id_str.clone()).or_default();
                                            let msg = format!("{} {} [Peer] {}", ts, latency, content);
                                            dm_msgs.push_back(msg.clone());
                                            if dm_msgs.len() > MAX_MESSAGES {
                                                dm_msgs.pop_front();
                                            }

                                            if !is_current_dm {
                                                *unread_dms.entry(peer_id_str.clone()).or_insert(0) += 1;
                                                dynamic_tabs.add_dm_tab(peer_id_str.clone());
                                            } else {
                                                log_debug(&logs, format!("Received DM from {}: {}", &peer_id_str[peer_id_str.len().saturating_sub(8.min(peer_id_str.len()))..], content));
                                            }

                                            if let Err(e) = save_message(&content, Some(&peer_id_str), &topic_str, true, Some(&peer_id_str)) {
                                                log_debug(&logs, format!("Failed to save DM: {}", e));
                                            }

                                            let response = DirectMessage {
                                                content: "ok".to_string(),
                                                timestamp: chrono::Utc::now().timestamp(),
                                                sent_at: Some(std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .expect("system time is valid")
                                                    .as_secs_f64()),
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
                                                    peers.push_front((peer_id_str.clone(), first_seen, last_seen));
                                                }
                                            }
                                            Err(e) => {
                                                if !peers.iter().any(|(id, _, _)| id == &peer_id_str) {
                                                    peers.push_front((peer_id_str.clone(), now_timestamp(), now_timestamp()));
                                                }
                                                log_debug(&logs, format!("Failed to save peer: {}", e));
                                            }
                                        }
                                        swarm.dial(multiaddr.clone()).ok();
                                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
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
                                        let (last_tcp_port, _last_quic_port) = load_listen_ports().unwrap_or((None, None));
                                        let _ = save_listen_ports(last_tcp_port, Some(port));
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
                                                peers.push_front((peer_id_str, first_seen, last_seen));
                                            }
                                        }
                                        Err(e) => {
                                            if !peers.iter().any(|(id, _, _)| id == &peer_id_str) {
                                                peers.push_front((peer_id_str.clone(), now_timestamp(), now_timestamp()));
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
                                                messages.push_back((retry_msg, None));
                                            }
                                        }
                                    }
                                }
                                SwarmEvent::ConnectionClosed {
                                    peer_id, cause: _, ..
                                } => {
                                    concurrent_peers = concurrent_peers.saturating_sub(1);
                                    log_debug(&logs, format!("Concurrent peers: {} (disconnected: {})", concurrent_peers, &peer_id.to_string()[peer_id.to_string().len().saturating_sub(8.min(peer_id.to_string().len()))..]));
                                    if let Err(e) = save_peer_session(concurrent_peers as i32) {
                                        log_debug(&logs, format!("Failed to save peer session: {}", e));
                                    }
                                }
                                SwarmEvent::Dialing { peer_id: Some(_pid), .. } => {}
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

                            if let Ok(l) = logs.lock()
                                && l.len() > MAX_LOGS {
                                    drop(l);
                                    if let Ok(mut l) = logs.lock() {
                                        l.pop_front();
                                    }
                                }
                        }

                        _ = tokio::time::sleep(Duration::from_millis(16)) => {
                            if poll(Duration::ZERO).ok() == Some(true)
                                && let Ok(event) = read()
                            {
                                match event {
                                    Event::Mouse(mouse) if mouse_capture
                                        && matches!(mouse.kind, crossterm::event::MouseEventKind::Down(_)) => {
                                        let row = mouse.row;
                                        let col = mouse.column as usize;

                                        if row == 0 {
                                            let titles = dynamic_tabs.all_titles();
                                            let mut char_pos = 0;
                                            for (i, title) in titles.iter().enumerate() {
                                                let title_len = title.len();
                                                let x_suffix = if title.ends_with(" (X)") { 4 } else { 0 };
                                                let clickable_len = title_len - x_suffix;
                                                if col >= char_pos && col <= char_pos + title_len {
                                                    if x_suffix > 0 && col > char_pos + clickable_len {
                                                        if let TabContent::Direct(t) = dynamic_tabs.tab_index_to_content(i) {
                                                            dynamic_tabs.remove_dm_tab(&t);
                                                            dm_messages.remove(&t);
                                                            dm_inputs.remove(&t);
                                                            unread_dms.remove(&t);
                                                            active_tab = 0;
                                                        }
                                                    } else {
                                                        active_tab = i;
                                                    }
                                                    break;
                                                }
                                                char_pos += title_len + 3;
                                            }
                                        } else if row == 3 && (unread_broadcasts > 0 || !unread_dms.is_empty()) {
                                            if unread_broadcasts > 0 && !unread_dms.is_empty() {
                                                let separator_pos = format!("{} broadcast(s) | ", unread_broadcasts).len();
                                                if col < separator_pos {
                                                    active_tab = 0;
                                                    unread_broadcasts = 0;
                                                } else {
                                                    if let Some((target, _)) = unread_dms.iter().next() {
                                                        let target_clone = target.clone();
                                                        active_tab = dynamic_tabs.add_dm_tab(target_clone.clone());
                                                        unread_dms.remove(&target_clone);
                                                    }
                                                }
                                            } else if unread_broadcasts > 0 {
                                                active_tab = 0;
                                                unread_broadcasts = 0;
                                            } else if let Some((target, _)) = unread_dms.iter().next() {
                                                let target_clone = target.clone();
                                                active_tab = dynamic_tabs.add_dm_tab(target_clone.clone());
                                                unread_dms.remove(&target_clone);
                                            }
                                        } else if matches!(dynamic_tabs.tab_index_to_content(active_tab), TabContent::Peers) {
                                            let tabs_rows = 3;
                                            let notification_rows = if unread_broadcasts > 0 || !unread_dms.is_empty() { 1 } else { 0 };
                                            let list_header_rows = 1;
                                            let peers_start_row = tabs_rows + notification_rows + list_header_rows;
                                            if row as usize >= peers_start_row {
                                                let p = row as usize - peers_start_row;
                                                if p < peers.len() {
                                                    peer_selection = p;
                                                    if let Some((pid, _, _)) = peers.get(p).cloned() {
                                                        active_tab = dynamic_tabs.add_dm_tab(pid.clone());
                                                        dm_messages.entry(pid).or_default();
                                                    }
                                                }
                                            }
                                        } else if matches!(dynamic_tabs.tab_index_to_content(active_tab), TabContent::Chat) {
                                            let term_width = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80);
                                            let content_width = term_width.saturating_sub(4);
                                            let tabs_rows = 3;
                                            let notification_rows = if unread_broadcasts > 0 || !unread_dms.is_empty() { 1 } else { 0 };
                                            let list_header_rows = 2;
                                            let content_start_row = tabs_rows + notification_rows + list_header_rows;

                                            if row >= content_start_row {
                                                let clicked_row = row - content_start_row;

                                                let mut current_row = 0;
                                                let mut msg_idx = chat_scroll_offset;

                                                while msg_idx < messages.len() {
                                                    let (msg_text, _) = &messages[msg_idx];
                                                    let manual_breaks = msg_text.matches('\n').count();
                                                    let wrapped_lines = (msg_text.len() / content_width).saturating_add(1);
                                                    let msg_lines = (manual_breaks + wrapped_lines).max(1);

                                                    if clicked_row >= current_row && clicked_row < current_row + msg_lines as u16 {
                                                        if let Some((_, Some(peer_id))) = messages.get(msg_idx) {
                                                            active_tab = dynamic_tabs.add_dm_tab(peer_id.clone());
                                                            dm_messages.entry(peer_id.clone()).or_default();
                                                        }
                                                        break;
                                                    }

                                                    current_row += msg_lines as u16;
                                                    msg_idx += 1;
                                                }
                                            }
                                        }
                                    }
                                    Event::Key(key) => {
                                        if key.code == KeyCode::F(12) {
                                            mouse_capture = !mouse_capture;
                                            if mouse_capture {
                                                execute!(std::io::stdout(), crossterm::event::EnableMouseCapture).ok();
                                            } else {
                                                execute!(std::io::stdout(), crossterm::event::DisableMouseCapture).ok();
                                            }
                                        } else if key.code == KeyCode::Esc
                                            || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q'))
                                        {
                                            execute!(std::io::stdout(), crossterm::event::DisableMouseCapture).ok();
                                            execute!(std::io::stdout(), PopKeyboardEnhancementFlags).ok();
                                            execute!(std::io::stdout(), LeaveAlternateScreen).ok();
                                            disable_raw_mode().ok();
                                            return Ok(());
                                        }

                                        let input: Input = key.into();
                                        let tab_content = dynamic_tabs.tab_index_to_content(active_tab);
                                        let tab_count = dynamic_tabs.total_tab_count();

                                        if key.code == KeyCode::Enter && key.modifiers.contains(KeyModifiers::ALT) {
                                            if tab_content.is_input_enabled() {
                                                match &tab_content {
                                                    TabContent::Chat => { chat_input.input_without_shortcuts(input); }
                                                    TabContent::Direct(peer_id) => {
                                                        let dm_input = dm_inputs.entry(peer_id.clone()).or_insert_with(|| {
                                                            let mut ta = TextArea::default();
                                                            ta.set_line_number_style(Style::default());
                                                            ta.set_cursor_line_style(Style::default());
                                                            ta
                                                        });
                                                        dm_input.input_without_shortcuts(input);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        } else if key.code == KeyCode::Enter {
                                            match tab_content {
                                                TabContent::Chat => {
                                                    let lines = chat_input.lines();
                                                    let text: String = lines.join("\n");
                                                    if !text.trim().is_empty() {
                                                        let ts = format_system_time(SystemTime::now());
                                                        let msg_str = format!("[{}] [You] {}", ts, text);
                                                        messages.push_back((msg_str.clone(), None));

                                                        let topic = gossipsub::IdentTopic::new("test-net");
                                                        log_debug(&logs, format!("Publishing to gossipsub topic: {}", topic));
                                                        let now = SystemTime::now();
                                                        let sent_at = now
                                                            .duration_since(std::time::UNIX_EPOCH)
                                                            .expect("system time is valid")
                                                            .as_secs_f64();
                                                        let broadcast = p2p_app::BroadcastMessage {
                                                            content: text.clone(),
                                                            sent_at: Some(sent_at),
                                                        };
                                                        let payload = serde_json::to_string(&broadcast).unwrap_or(text.clone());
                                                        let publish_result = swarm.behaviour_mut().gossipsub.publish(
                                                            topic,
                                                            payload.as_bytes(),
                                                        );

                                                        if let Err(e) = publish_result {
                                                            log_debug(&logs, format!("Publish error: {:?}", e));
                                                        } else {
                                                            log_debug(&logs, "Message published successfully".to_string());
                                                        }

                                                        if let Err(e) = save_message(&text, None, &topic_str, false, None) {
                                                            log_debug(&logs, format!("Failed to save message: {}", e));
                                                        }
                                                    }
                                                    chat_input = TextArea::default();
                                                    chat_input.set_line_number_style(Style::default());
                                                    chat_input.set_cursor_line_style(Style::default());
                                                }
                                                TabContent::Peers if !peers.is_empty() => {
                                                    let idx = peer_selection.min(peers.len() - 1);
                                                    if let Some(peer) = peers.get(idx).cloned() {
                                                        let (peer_id, _, _) = peer;
                                                        active_tab = dynamic_tabs.add_dm_tab(peer_id.clone());
                                                        let dm_msgs = dm_messages.entry(peer_id.clone()).or_default();
                                                        dm_msgs.clear();
                                                        if let Ok(msgs) = load_direct_messages(&peer_id, MAX_MESSAGES) {
                                                            for msg in msgs {
                                                                let ts = format_peer_datetime(msg.created_at);
                                                                let sender = if msg.peer_id.is_some() { "[You]" } else { "[Peer]" };
                                                                dm_msgs.push_back(format!("[{}] {} {}", ts, sender, msg.content));
                                                            }
                                                        }
                                                    }
                                                }
                                                TabContent::Direct(target) => {
                                                    let dm_input = dm_inputs.entry(target.clone()).or_insert_with(|| {
                                                        let mut ta = TextArea::default();
                                                        ta.set_line_number_style(Style::default());
                                                        ta.set_cursor_line_style(Style::default());
                                                        ta
                                                    });
                                                    let lines = dm_input.lines();
                                                    let text: String = lines.join("\n");
                                                    if !text.trim().is_empty() {
                                                        let ts = format_system_time(SystemTime::now());
                                                        let msg_str = format!("[{}] [You] {}", ts, text);
                                                        let dm_msgs = dm_messages.entry(target.clone()).or_default();
                                                        dm_msgs.push_back(msg_str.clone());
                                                        log_debug(&logs, format!("Sending DM to {}", target));

                                                        let peer_id: libp2p::PeerId = match target.parse() {
                                                            Ok(pid) => pid,
                                                            Err(e) => {
                                                                log_debug(&logs, format!("Invalid peer ID: {}", e));
                                                                *dm_inputs.get_mut(&target.clone()).unwrap() = TextArea::default();
                                                                dm_inputs.get_mut(&target).unwrap().set_line_number_style(Style::default());
                                                                dm_inputs.get_mut(&target).unwrap().set_cursor_line_style(Style::default());
                                                                continue;
                                                            }
                                                        };

                                                        let dm = DirectMessage {
                                                            content: text.clone(),
                                                            timestamp: chrono::Utc::now().timestamp(),
                                                            sent_at: Some(std::time::SystemTime::now()
                                                                .duration_since(std::time::UNIX_EPOCH)
                                                                .expect("system time is valid")
                                                                .as_secs_f64()),
                                                        };

                                                        swarm.behaviour_mut().request_response.send_request(&peer_id, dm);
                                                        log_debug(&logs, format!("DM request sent to {}", target));

                                                        if let Err(e) = save_message(&text, None, &topic_str, true, Some(&peer_id.to_string())) {
                                                            log_debug(&logs, format!("Failed to save DM: {}", e));
                                                        }
                                                    }
                                                    *dm_inputs.get_mut(&target.clone()).unwrap() = TextArea::default();
                                                    dm_inputs.get_mut(&target).unwrap().set_line_number_style(Style::default());
                                                    dm_inputs.get_mut(&target).unwrap().set_cursor_line_style(Style::default());
                                                }
                                                _ => {}
                                            }
                                        } else if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('w') {
                                            if let TabContent::Direct(target) = tab_content {
                                                dynamic_tabs.remove_dm_tab(&target);
                                                dm_messages.remove(&target);
                                                dm_inputs.remove(&target);
                                                unread_dms.remove(&target);
                                                active_tab = 0;
                                            }
                                        } else if key.modifiers.contains(KeyModifiers::CONTROL)
                                            && (key.code == KeyCode::Char('n') || key.code == KeyCode::Char('g'))
                                        {
                                            if unread_broadcasts > 0 {
                                                active_tab = 0;
                                                unread_broadcasts = 0;
                                            } else if let Some((target, _)) = unread_dms.iter().next() {
                                                let target_clone = target.clone();
                                                active_tab = dynamic_tabs.add_dm_tab(target_clone.clone());
                                                unread_dms.remove(&target_clone);
                                            }
                                        } else if key.code == KeyCode::Tab || key.code == KeyCode::BackTab {
                                            if key.code == KeyCode::BackTab {
                                                active_tab = if active_tab == 0 { tab_count - 1 } else { active_tab - 1 };
                                            } else {
                                                active_tab = (active_tab + 1) % tab_count;
                                            }
                                            let new_content = dynamic_tabs.tab_index_to_content(active_tab);
                                            if matches!(new_content, TabContent::Chat) {
                                                unread_broadcasts = 0;
                                            }
                                            if let TabContent::Direct(target) = new_content {
                                                unread_dms.remove(&target);
                                            }
                                        } else if matches!(tab_content, TabContent::Chat) {
                                            match key.code {
                                                KeyCode::Up => {
                                                    chat_auto_scroll = false;
                                                    chat_scroll_offset = chat_scroll_offset.saturating_sub(1);
                                                }
                                                KeyCode::Down => {
                                                    chat_scroll_offset = (chat_scroll_offset + 1).min(messages.len().saturating_sub(1));
                                                }
                                                KeyCode::PageUp => {
                                                    chat_auto_scroll = false;
                                                    chat_scroll_offset = chat_scroll_offset.saturating_sub(20);
                                                }
                                                KeyCode::PageDown => {
                                                    chat_scroll_offset = (chat_scroll_offset + 20).min(messages.len().saturating_sub(1));
                                                }
                                                KeyCode::End => {
                                                    chat_scroll_offset = usize::MAX;
                                                    chat_auto_scroll = true;
                                                }
                                                KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
                                                | KeyCode::Left | KeyCode::Right | KeyCode::Home
                                                | KeyCode::Enter => {
                                                    chat_input.input(input);
                                                }
                                                _ => {}
                                            }
                                        } else if matches!(tab_content, TabContent::Peers) {
                                            match key.code {
                                                KeyCode::Up => {
                                                    if !peers.is_empty() {
                                                        peer_selection = peer_selection.saturating_sub(1);
                                                    }
                                                }
                                                KeyCode::Down => {
                                                    if !peers.is_empty() {
                                                        peer_selection = (peer_selection + 1).min(peers.len() - 1);
                                                    }
                                                }
                                                _ => {}
                                            }
                                        } else if let TabContent::Direct(peer_id) = tab_content {
                                            let dm_input = dm_inputs.entry(peer_id.to_string()).or_insert_with(|| {
                                                let mut ta = TextArea::default();
                                                ta.set_line_number_style(Style::default());
                                                ta.set_cursor_line_style(Style::default());
                                                ta
                                            });
                                            match key.code {
                                                KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
                                                | KeyCode::Left | KeyCode::Right | KeyCode::Home
                                                | KeyCode::End | KeyCode::Enter => {
                                                    dm_input.input(input);
                                                }
                                                _ => {}
                                            }
                                        } else if matches!(tab_content, TabContent::Log) {
                                            match key.code {
                                                KeyCode::Up => {
                                                    debug_auto_scroll = false;
                                                    debug_scroll_offset = debug_scroll_offset.saturating_sub(1);
                                                }
                                                KeyCode::Down => {
                                                    if let Ok(l) = logs.lock() {
                                                        debug_scroll_offset = (debug_scroll_offset + 1).min(l.len().saturating_sub(1));
                                                    }
                                                }
                                                KeyCode::PageUp => {
                                                    debug_auto_scroll = false;
                                                    debug_scroll_offset = debug_scroll_offset.saturating_sub(20);
                                                }
                                                KeyCode::PageDown => {
                                                    if let Ok(l) = logs.lock() {
                                                        debug_scroll_offset = (debug_scroll_offset + 20).min(l.len().saturating_sub(1));
                                                    }
                                                }
                                                KeyCode::End => {
                                                    debug_scroll_offset = usize::MAX;
                                                    debug_auto_scroll = true;
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }

                    } // close Event::Key arm
                    terminal.draw(|f| {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(1),
                                Constraint::Length(1),
                                Constraint::Min(0),
                                Constraint::Length(5),
                                Constraint::Length(1),
                            ])
                            .split(f.area());

                        let tab_titles = dynamic_tabs.all_titles();
                        let tabs = Tabs::new(tab_titles.clone())
                            .style(Style::default().fg(Color::Cyan))
                            .select(active_tab);
                        f.render_widget(tabs, chunks[0]);

                        if unread_broadcasts > 0 || !unread_dms.is_empty() {
                            let mut parts = Vec::new();
                            if unread_broadcasts > 0 {
                                parts.push(format!("{} broadcast(s)", unread_broadcasts));
                            }
                            if !unread_dms.is_empty() {
                                let total: u32 = unread_dms.values().sum();
                                parts.push(format!(
                                    "{} DM(s) from {} peer(s)",
                                    total,
                                    unread_dms.len()
                                ));
                            }
                            let notif_text = Paragraph::new(parts.join(" | "))
                                .style(Style::default().fg(Color::Yellow));
                            f.render_widget(notif_text, chunks[1]);
                        }

                        let content_area = chunks[2];

                        match dynamic_tabs.tab_index_to_content(active_tab) {
                            TabContent::Chat => {
                                let total = messages.len();
                                let visible_height = content_area.height.saturating_sub(2) as usize;

                                if chat_auto_scroll {
                                    chat_scroll_offset = total.saturating_sub(visible_height);
                                }

                                if chat_scroll_offset > total.saturating_sub(1) {
                                    chat_scroll_offset = total.saturating_sub(1);
                                }

                                let chat_title =
                                    format!("Messages [{}/{}]", chat_scroll_offset + 1, total);
                                let chat_items: Vec<ListItem> = messages
                                    .iter()
                                    .skip(chat_scroll_offset)
                                    .map(|(m, _)| ListItem::new(m.clone()))
                                    .collect();
                                let chat_list = List::new(chat_items)
                                    .block(Block::default().title(chat_title).borders(Borders::ALL))
                                    .style(Style::default().fg(Color::White));
                                f.render_widget(chat_list, content_area);
                            }
                            TabContent::Peers => {
                                let peer_items: Vec<ListItem> = peers
                                    .iter()
                                    .enumerate()
                                    .map(|(i, (peer_id, first_seen, last_seen))| {
                                        let prefix = if i == peer_selection { ">> " } else { "   " };
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
                                f.render_widget(peer_list, content_area);
                            }
                            TabContent::Direct(peer_id) => {
                                let peer_name: String = peer_id
                                    .chars()
                                    .rev()
                                    .take(8)
                                    .collect::<String>()
                                    .chars()
                                    .rev()
                                    .collect();
                                let dm_msgs = dm_messages.entry(peer_id.clone()).or_default();
                                let dm_items: Vec<ListItem> = dm_msgs
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
                                f.render_widget(dm_list, content_area);
                            }
                            TabContent::Log => {
                                let log_vec = logs.lock().expect("logs mutex not poisoned").clone();
                                let total = log_vec.len();
                                let visible_height = content_area.height.saturating_sub(2) as usize;

                                if debug_auto_scroll {
                                    debug_scroll_offset = total.saturating_sub(visible_height);
                                }

                                if debug_scroll_offset > total.saturating_sub(1) {
                                    debug_scroll_offset = total.saturating_sub(1);
                                }

                                let log_items: Vec<ListItem> = log_vec
                                    .iter()
                                    .skip(debug_scroll_offset)
                                    .map(|l| ListItem::new(l.clone()))
                                    .collect();

                                let log_title =
                                    format!("Log [{}/{}]", debug_scroll_offset + 1, total);
                                let log_list = List::new(log_items)
                                    .block(
                                        Block::default().title(log_title).borders(Borders::ALL),
                                    )
                                    .style(Style::default().fg(Color::White));
                                f.render_widget(log_list, content_area);
                            }
                        }

                        let tab_content = dynamic_tabs.tab_index_to_content(active_tab);
                        let input_area = if tab_content.is_input_enabled() {
                            chunks[3]
                        } else {
                            ratatui::layout::Rect::default()
                        };
                        if !input_area.is_empty() {
                            match &tab_content {
                                TabContent::Chat => {
                                    let textarea_block = Block::default()
                                        .title("Input (Enter to send, Alt+Enter for newline)")
                                        .borders(Borders::ALL);
                                    let mut textarea_clone = chat_input.clone();
                                    textarea_clone.set_block(textarea_block);
                                    f.render_widget(&textarea_clone, input_area);
                                }
                                TabContent::Direct(peer_id) => {
                                    let dm_input = dm_inputs.entry(peer_id.clone()).or_insert_with(|| {
                                        let mut ta = TextArea::default();
                                        ta.set_line_number_style(Style::default());
                                        ta.set_cursor_line_style(Style::default());
                                        ta
                                    });
                                    let textarea_block = Block::default()
                                        .title("DM Input (Enter to send, Alt+Enter for newline)")
                                        .borders(Borders::ALL);
                                    let mut textarea_clone = dm_input.clone();
                                    textarea_clone.set_block(textarea_block);
                                    f.render_widget(&textarea_clone, input_area);
                                }
                                _ => {}
                            }
                        }

                        let help = Paragraph::new(
                            "Tab: cycle | Enter: send | Ctrl+G: notification | Ctrl+W: close | F12: mouse | PgUp/PgDn: scroll | Ctrl+Q: quit",
                        )
                        .style(Style::default().fg(Color::DarkGray));
                        f.render_widget(help, chunks[4]);
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
                            concurrent_peers = concurrent_peers.saturating_sub(1);
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

    let listen_addr: libp2p::Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    swarm.listen_on(listen_addr)?;

    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    tui::run_tui(swarm, "test-net".to_string(), logs).await
}
