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
        ensure_self_nickname, format_peer_datetime, get_database_url, get_network_size,
         get_self_nickname, get_unsent_messages, load_direct_messages,
         load_listen_ports, load_messages, load_peers, mark_message_sent, now_timestamp,
         save_listen_ports, save_message, save_peer, save_peer_session,
         set_peer_local_nickname, set_self_nickname,
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
        use std::{collections::VecDeque, time::SystemTime};

        fn format_system_time(time: SystemTime) -> String {
            chrono::DateTime::<chrono::Local>::from(time)
                .format("%H:%M:%S.%3f")
                .to_string()
        }

        #[derive(Clone)]
        pub struct TracingWriter {
            logs: std::sync::Arc<std::sync::Mutex<VecDeque<String>>>,
        }

        impl TracingWriter {
            #[must_use]
            pub fn new(logs: std::sync::Arc<std::sync::Mutex<VecDeque<String>>>) -> Self {
                Self { logs }
            }
        }

        impl std::io::Write for TracingWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if let Ok(s) = std::str::from_utf8(buf) {
                     let cleaned = p2p_app::logging::strip_ansi_codes(s);
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
        chrono::DateTime::<chrono::Local>::from(time)
            .format("%H:%M:%S.%3f")
            .to_string()
    }

    fn short_peer_id(id: &str) -> String {
        id[id.len().saturating_sub(8.min(id.len()))..].to_string()
    }

    fn peer_display_name(
        peer_id: &str,
        local_nicknames: &HashMap<String, String>,
        received_nicknames: &HashMap<String, String>,
    ) -> String {
        if let Some(nick) = local_nicknames.get(peer_id) {
            let short = short_peer_id(peer_id);
            return format!("{} ({})", nick, &short[..3.min(short.len())]);
        }
        if let Some(nick) = received_nicknames.get(peer_id) {
            let short = short_peer_id(peer_id);
            return format!("{} ({})", nick, &short[..3.min(short.len())]);
        }
        short_peer_id(peer_id)
    }

    fn clamp_scroll(total: usize, current: usize, delta: isize) -> usize {
        if delta < 0 {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(total.saturating_sub(1))
        }
    }

    fn auto_scroll_offset(total: usize, visible: usize) -> usize {
        total.saturating_sub(visible).min(total.saturating_sub(1))
    }

    fn scroll_title(prefix: &str, scroll_offset: usize, total: usize) -> String {
        format!("{} [{}/{}]", prefix, scroll_offset + 1, total)
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

    fn style_textarea(ta: &mut TextArea) {
        ta.set_line_number_style(Style::default());
        ta.set_cursor_line_style(Style::default());
    }

    fn init_textarea() -> TextArea<'static> {
        let mut ta = TextArea::default();
        style_textarea(&mut ta);
        ta
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
                let mut chat_input = init_textarea();
                let mut concurrent_peers: usize = 0;
                let mut peer_selection: usize = 0;
                let mut debug_scroll_offset: usize = 0;
                let mut debug_auto_scroll: bool = true;
                let mut chat_scroll_offset: usize = 0;
                let mut chat_auto_scroll: bool = true;
                let mut own_nickname: String = ensure_self_nickname().unwrap_or_else(|_| {
                    log_debug(&logs, "Failed to get/ensure nickname".to_string());
                    "unknown".to_string()
                });
                let mut local_nicknames: HashMap<String, String> = HashMap::new();
                let mut received_nicknames: HashMap<String, String> = HashMap::new();
                log_debug(&logs, format!("Your nickname: {}", own_nickname));

                init_logging();
                let logs_for_callback = logs.clone();
                 p2p_app::logging::set_tui_log_callback(move |msg| {
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
                            .map(|p| {
                                let display =
                                    peer_display_name(p, &local_nicknames, &received_nicknames);
                                format!("[{}]", display)
                            })
                            .unwrap_or_else(|| format!("[{}]", own_nickname));
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
                        if let Some(ref local_nick) = peer.peer_local_nickname {
                            local_nicknames.insert(peer.peer_id.clone(), local_nick.clone());
                        }
                        if let Some(ref recv_nick) = peer.received_nickname {
                            received_nicknames.insert(peer.peer_id.clone(), recv_nick.clone());
                        }
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
        // Handle swarm events - simplified without tokio::select!
        let event = swarm.select_next_some().await;
        match event {
            SwarmEvent::Behaviour(AppEv::Gossipsub(gossipsub::Event::Message { propagation_source: peer_id, message, .. })) => {
                let raw = String::from_utf8_lossy(&message.data).to_string();
                let now = std::time::SystemTime::now();
                let ts = format_latency(None, now);
                let (content, sent_at, _received_nick) = match serde_json::from_str::<p2p_app::BroadcastMessage>(&raw) {
                    Ok(bcast) => {
                        if let Some(nick) = &bcast.nickname {
                            state.received_nicknames.insert(peer_id.to_string(), nick.clone());
                            p2p_app::set_peer_received_nickname(&peer_id.to_string(), nick).ok();
                            state.rendered_messages = p2p_app::render_all_messages(
                                &state.messages, &state.own_nickname, &state.local_nicknames, &state.received_nicknames,
                            );
                        }
                        (bcast.content, bcast.sent_at, bcast.nickname)
                    }
                    Err(_) => (raw, None, None),
                };
                let sender_display = peer_display_name(&peer_id.to_string(), &state.local_nicknames, &state.received_nicknames);
                let latency = format_latency(sent_at, now);
                let msg = format!("{} {} [{}] {}", ts, latency, sender_display, content);
                state.messages.push_back((msg.clone(), p2p_app::RawChatMessage {
                    timestamp: format!("{} {}", ts, latency),
                    source: p2p_app::ChatMessageSource::Peer(peer_id.to_string()),
                    content: content.clone(),
                }));
                if state.messages.len() > 1000 {
                    state.messages.pop_front();
                    state.rendered_messages.pop_front();
                }
                if state.active_tab != 0 {
                    state.unread_broadcasts += 1;
                }
                log_debug(logs, format!("[{}] {}", ts, content));
                p2p_app::save_message(&content, Some(&peer_id), &topic_str, false, None).ok();
            }
            // ... other arms would go here ...
            _ => {}
        }
    }

    async fn run_headless_mode