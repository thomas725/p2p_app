use libp2p::{gossipsub, noise, tcp, yamux};
use p2p_app::logging::init_logging;
use p2p_app::{
    AppBehaviour, AppBehaviourEvent as AppEv, DirectMessage, DynamicTabs, NetworkSize, TabContent,
    TuiEvent, auto_scroll_offset, build_behaviour, create_channels, ensure_self_nickname,
    format_latency, format_peer_datetime, format_system_time, get_database_url, get_network_size,
    get_self_nickname, get_unsent_messages, load_direct_messages, load_listen_ports, load_messages,
    load_peers, log_debug, mark_message_sent, now_timestamp, peer_display_name, save_listen_ports,
    save_message, save_peer, save_peer_session, scroll_title, set_peer_local_nickname,
    set_self_nickname, short_peer_id,
};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

#[cfg(feature = "tui")]
mod tui {
    use super::*;
    use libp2p::Swarm;
    #[cfg(feature = "mdns")]
    use libp2p::mdns;
    use libp2p::{futures::StreamExt, gossipsub, swarm::SwarmEvent as Libp2pSwarmEvent};
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
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tracing_subscriber::prelude::*;

    mod state;
    mod tracing_writer;
    mod command_processor;
    mod input_handler;
    mod render_loop;
    pub mod main_loop;

    const MAX_MESSAGES: usize = 1000;
    const MAX_LOGS: usize = 1000;

    #[derive(Debug)]
    pub enum UiCommand {
        ShowMessage(String, Option<String>),
        BroadcastMessage(String, String, Option<String>),
        DirectMessage(String, String, Option<String>),
        UpdatePeers(VecDeque<(String, String, String)>),
        PeerConnected(String),
        PeerDisconnected(String),
        NewListenAddr(String),
        AddDmTab(String),
        RemoveDmTab(String),
        SetActiveTab(usize),
        ShowDmMessages(String, VecDeque<String>),
        ClearInput,
        SetChatInput(String),
        ToggleMouseCapture,
        Exit,
    }

    #[derive(Debug)]
    pub enum SwarmCommand {
        Publish(String),
        SendDm(String, String),
    }

    pub type SwarmCommandTx = mpsc::Sender<SwarmCommand>;
    pub type SwarmCommandRx = mpsc::Receiver<SwarmCommand>;

    #[derive(Debug)]
    pub enum InputEvent {
        Key(Event),
        Mouse(Event),
    }

    pub type UiCommandTx = mpsc::Sender<UiCommand>;
    pub type UiCommandRx = mpsc::Receiver<UiCommand>;
    pub type InputEventTx = mpsc::Sender<InputEvent>;
    pub type InputEventRx = mpsc::Receiver<InputEvent>;

    fn style_textarea(ta: &mut TextArea) {
        ta.set_line_number_style(Style::default());
        ta.set_cursor_line_style(Style::default());
    }

    fn init_textarea() -> TextArea<'static> {
        let mut ta = TextArea::default();
        style_textarea(&mut ta);
        ta
    }

    fn exit_tui() -> color_eyre::Result<()> {
        use crossterm::event::{DisableMouseCapture, PopKeyboardEnhancementFlags};
        use ratatui::crossterm::terminal::disable_raw_mode;
        use ratatui::crossterm::{execute, terminal::LeaveAlternateScreen};
        execute!(std::io::stdout(), DisableMouseCapture).ok();
        execute!(std::io::stdout(), PopKeyboardEnhancementFlags).ok();
        execute!(std::io::stdout(), LeaveAlternateScreen).ok();
        disable_raw_mode().ok();
        Ok(())
    }

    fn handle_chat_input(
        text: &str,
        swarm: &mut Swarm<AppBehaviour>,
        topic: &gossipsub::IdentTopic,
        own_nickname: &str,
        topic_str: &str,
        logs: &Arc<Mutex<VecDeque<String>>>,
    ) -> Option<String> {
        if text.starts_with('/') {
            let parts: Vec<&str> = text.splitn(2, ' ').collect();
            match parts[0] {
                "/nick" => {
                    if let Some(new_nick) = parts.get(1) {
                        let new_nick = new_nick.trim();
                        if !new_nick.is_empty() {
                            if let Err(e) = set_self_nickname(new_nick) {
                                log_debug(logs, format!("Failed to set nickname: {}", e));
                            } else {
                                let ts = format_system_time(SystemTime::now());
                                return Some(format!(
                                    "{} [System] Nickname changed to {}",
                                    ts, new_nick
                                ));
                            }
                        }
                    } else if let Ok(Some(current)) = get_self_nickname() {
                        log_debug(logs, format!("Current nickname: {}", current));
                    }
                    return None;
                }
                "/setname" => {
                    let parts: Vec<&str> = text.splitn(3, ' ').collect();
                    if parts.len() >= 3 {
                        let target_peer = parts[1];
                        let new_nick = parts[2].trim();
                        if !new_nick.is_empty() {
                            if let Err(e) = set_peer_local_nickname(target_peer, new_nick) {
                                log_debug(logs, format!("Failed to set peer nickname: {}", e));
                            } else {
                                log_debug(
                                    logs,
                                    format!("Set local nickname for {}: {}", target_peer, new_nick),
                                );
                                return Some(format!(
                                    "Set nickname for {}: {}",
                                    short_peer_id(target_peer),
                                    new_nick
                                ));
                            }
                        }
                    }
                    return None;
                }
                "/help" | "/h" => {
                    return Some(
                        "Commands: /nick [name], /setname <peer> <name>, /help".to_string(),
                    );
                }
                _ => {
                    log_debug(logs, "Commands: /nick [name], /setname".to_string());
                    return None;
                }
            }
        }
        None
    }

    fn handle_send_broadcast(
        text: &str,
        swarm: &mut Swarm<AppBehaviour>,
        topic: &gossipsub::IdentTopic,
        own_nickname: &str,
        topic_str: &str,
        logs: &Arc<Mutex<VecDeque<String>>>,
    ) -> Option<String> {
        let ts = format_system_time(SystemTime::now());
        let msg_str = format!("[{}] [You] {}", ts, text);
        let payload = text.to_string();
        if let Err(e) = swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), payload.as_bytes())
        {
            log_debug(logs, format!("Publish error: {:?}", e));
        } else {
            log_debug(logs, "Message published");
        }
        let _ = save_message(text, None, topic_str, false, None);
        Some(msg_str)
    }

    fn handle_send_dm(
        target: &str,
        text: &str,
        swarm: &mut Swarm<AppBehaviour>,
        own_nickname: &str,
        topic_str: &str,
        logs: &Arc<Mutex<VecDeque<String>>>,
    ) -> Option<String> {
        let peer_id: libp2p::PeerId = match target.parse() {
            Ok(pid) => pid,
            Err(e) => {
                log_debug(logs, format!("Invalid peer ID: {}", e));
                return None;
            }
        };
        let ts = format_system_time(SystemTime::now());
        let msg_str = format!("[{}] [You] {}", ts, text);
        let dm = DirectMessage {
            content: text.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            sent_at: Some(
                SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system time valid")
                    .as_secs_f64(),
            ),
            nickname: Some(own_nickname.to_string()),
        };
        swarm
            .behaviour_mut()
            .request_response
            .send_request(&peer_id, dm);
        let _ = save_message(text, None, topic_str, true, Some(&peer_id.to_string()));
        Some(msg_str)
    }

    pub async fn run_tui(
        mut swarm: Swarm<AppBehaviour>,
        topic_str: String,
        logs: Arc<Mutex<VecDeque<String>>>,
    ) -> color_eyre::Result<()> {
        let is_tty = atty::is(atty::Stream::Stdout);
        if !is_tty {
            color_eyre::eyre::bail!("Not a TTY. Use bin/p2p_chat for non-interactive mode");
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

                p2p_app::logging::init_logging();
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
                        tui::tracing_writer::TracingWriter::new(logs_for_tracing.clone())
                    })
                    .compact()
                    .with_filter(p2p_app::tracing_filter());
                let _ = tracing_subscriber::registry().with(trace_layer).try_init();
                log_debug(&logs, format!("Using database: {}", get_database_url()));
                log_debug(&logs, format!("Our peer ID: {}", swarm.local_peer_id()));

                // Create channels for async task communication
                let (ui_command_tx, mut ui_command_rx) = mpsc::channel(100);
                let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(100);
                let input_tx = ui_command_tx.clone();

                // Spawn input handler task (runs concurrently)
                let input_handler_handle = tokio::spawn(async move {
                    use std::time::Duration;
                    loop {
                        tokio::time::sleep(Duration::from_millis(16)).await;
                        if poll(Duration::ZERO).ok() == Some(true) {
                            if let Ok(event) = read() {
                                match event {
                                    Event::Key(key) => {
                                        if key.code == KeyCode::Esc
                                            || (key.modifiers.contains(KeyModifiers::CONTROL)
                                                && key.code == KeyCode::Char('q'))
                                        {
                                            let _ = input_tx.send(UiCommand::Exit).await;
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                });

                let topic = gossipsub::IdentTopic::new("test-net");

                let swarm_handler_handle: tokio::task::JoinHandle<()> =
                    tokio::spawn(async move { futures::future::pending::<()>().await });

                let loaded_messages = tui::state::load_and_format_messages(
                    &topic_str,
                    MAX_MESSAGES,
                    &logs,
                    &local_nicknames,
                    &received_nicknames,
                    &own_nickname,
                );
                messages.extend(loaded_messages);

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
                    // Handle outbound commands
                    if let Ok(cmd) = swarm_cmd_rx.try_recv() {
                        match cmd {
                            SwarmCommand::Publish(content) => {
                                let bcast = p2p_app::BroadcastMessage {
                                    content: content.clone(),
                                    sent_at: Some(
                                        std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .expect("system time is valid")
                                            .as_secs_f64(),
                                    ),
                                    nickname: Some(own_nickname.clone()),
                                };
                                if let Ok(msg) = serde_json::to_string(&bcast) {
                                    if let Err(e) = swarm
                                        .behaviour_mut()
                                        .gossipsub
                                        .publish(topic.clone(), msg.as_bytes())
                                    {
                                        log_debug(&logs, format!("Broadcast failed: {}", e));
                                    }
                                }
                            }
                            SwarmCommand::SendDm(_peer_id, _content) => {}
                        }
                    }

                    // Handle UI commands via ready!
                    tokio::select! {
                        biased;

                        cmd = ui_command_rx.recv() => {
                                                match cmd {
                                                    Some(UiCommand::ShowMessage(msg, peer_id)) => {
                                                        messages.push_back((msg, peer_id));
                                                        if messages.len() > MAX_MESSAGES {
                                                            messages.pop_front();
                                                        }
                                                    }
                                                    Some(UiCommand::Exit) => {
                                                        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
                                                        let _ = disable_raw_mode();
                                                        let _ = execute!(std::io::stdout(), PopKeyboardEnhancementFlags);
                                                        let _ = execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
                                                        swarm_handler_handle.abort();
                                                        input_handler_handle.abort();
                                                        return Ok(());
                                                    }
                                                    Some(UiCommand::AddDmTab(target)) => {
                                                        dynamic_tabs.add_dm_tab(target);
                                                    }
                                                    Some(UiCommand::PeerConnected(peer_id)) => {
                                                        concurrent_peers += 1;
                                                        let addresses = vec![peer_id.clone()];
                                                        if let Ok(peer) = save_peer(&peer_id, &addresses) {
                                                            let first_seen = format_peer_datetime(peer.first_seen);
                                                            let last_seen = format_peer_datetime(peer.last_seen);
                                                            if !peers.iter().any(|(id, _, _)| id == &peer_id) {
                                                                peers.push_front((peer_id.clone(), first_seen, last_seen));
                                                            }
                                                        }
                                                        if let Ok(peer_id_val) = peer_id.parse() {
                                                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id_val);
                                                        }
                                                    }
                                                    Some(UiCommand::PeerDisconnected(peer_id)) => {
                                                        concurrent_peers = concurrent_peers.saturating_sub(1);
                                                    }
                                                    Some(UiCommand::NewListenAddr(addr)) => {
                                                        log_debug(&logs, format!("Listening on: {}", addr));
                                                    }
                                                    Some(UiCommand::BroadcastMessage(content, peer_id_str, latency)) => {
                                                        let now = SystemTime::now();
                                                        let ts = format_system_time(now);
                                                        let sender_display = peer_display_name(&peer_id_str, &local_nicknames, &received_nicknames);
                                                        let msg = format!("{} {} [{}] {}", ts, latency.unwrap_or_default(), sender_display, content);
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
                                                    Some(UiCommand::DirectMessage(content, peer_id_str, latency)) => {
                                                        let now = SystemTime::now();
                                                        let ts = format_system_time(now);
                                                        let sender_display = peer_display_name(&peer_id_str, &local_nicknames, &received_nicknames);
                                                        let dm_msgs = dm_messages.entry(peer_id_str.clone()).or_default();
                                                        let msg = format!("{} {} [{}] {}", ts, latency.unwrap_or_default(), sender_display, content);
                                                        dm_msgs.push_back(msg.clone());
                                                        if dm_msgs.len() > MAX_MESSAGES {
                                                            dm_msgs.pop_front();
                                                        }
                                                        let current_peer = match dynamic_tabs.tab_index_to_content(active_tab) {
                                                            TabContent::Direct(id) => Some(id),
                                                            _ => None,
                                                        };
                                                        let is_current_dm = current_peer.as_ref() == Some(&peer_id_str);
                                                        if !is_current_dm {
                                                            *unread_dms.entry(peer_id_str.clone()).or_insert(0) += 1;
                                                            dynamic_tabs.add_dm_tab(peer_id_str.clone());
                                                        }
                                                        if let Err(e) = save_message(&content, Some(&peer_id_str), &topic_str, true, Some(&peer_id_str)) {
                                                            log_debug(&logs, format!("Failed to save DM: {}", e));
                                                        }
                                                    }
                                                    Some(UiCommand::UpdatePeers(new_peers)) => {
                                                        for (peer_id, first_seen, last_seen) in new_peers {
                                                            if !peers.iter().any(|(id, _, _)| id == &peer_id) {
                                                                peers.push_front((peer_id, first_seen, last_seen));
                                                            }
                                                        }
                                                    }
                                                    Some(UiCommand::ShowDmMessages(peer_id, dm_msgs)) => {
                                                        dynamic_tabs.add_dm_tab(peer_id.clone());
                                                        dm_messages.insert(peer_id, dm_msgs);
                                                    }
                                                    Some(UiCommand::RemoveDmTab(peer_id)) => {
                                                        dynamic_tabs.remove_dm_tab(&peer_id);
                                                        dm_messages.remove(&peer_id);
                                                        dm_inputs.remove(&peer_id);
                                                    }
                                                    Some(UiCommand::SetActiveTab(idx)) => {
                                                        active_tab = idx;
                                                    }
                                                    Some(UiCommand::ClearInput) => {
                                                        chat_input = init_textarea();
                                                    }
                                                    Some(UiCommand::SetChatInput(text)) => {
                                                        chat_input = init_textarea();
                                                        chat_input.insert_str(&text);
                                                    }
                                                    Some(UiCommand::ToggleMouseCapture) => {
                                                        mouse_capture = !mouse_capture;
                                                        if mouse_capture {
                                                            let _ = execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
                                                        } else {
                                                            let _ = execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }

                                            event = swarm.select_next_some() => {
                                                match event {
                                                    Libp2pSwarmEvent::Behaviour(AppEv::Gossipsub(
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

                                                        let (content, sent_at, _received_nick) = if let Ok(bcast) = serde_json::from_str::<p2p_app::BroadcastMessage>(&raw) {
                                                            if let Some(nick) = &bcast.nickname {
                                                                received_nicknames.insert(peer_id_str.clone(), nick.clone());
                                                                let _ = p2p_app::set_peer_received_nickname(&peer_id_str, nick);
                                                            }
                                                            (bcast.content, bcast.sent_at, bcast.nickname.clone())
                                                        } else {
                                                            (raw, None, None)
                                                        };

                                                        let sender_display = peer_display_name(&peer_id_str, &local_nicknames, &received_nicknames);
                                                        let latency = format_latency(sent_at, now);
                                                        let msg = format!("{} {} [{}] {}", ts, latency, sender_display, content);
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
                                                    Libp2pSwarmEvent::Behaviour(AppEv::RequestResponse(
                                                        libp2p::request_response::Event::Message { peer, message, .. },
                                                    )) => {
                                                        match message {
                                                            libp2p::request_response::Message::Request { request, channel, .. } => {
                                                                let peer_id_str = peer.to_string();
                                                                let content = request.content.clone();
                                                                let now = SystemTime::now();
                                                                let ts = format_system_time(now);
                                                                let latency = format_latency(request.sent_at, now);

                                                                if let Some(ref nick) = request.nickname {
                                                                    received_nicknames.insert(peer_id_str.clone(), nick.clone());
                                                                    let _ = p2p_app::set_peer_received_nickname(&peer_id_str, nick);
                                                                }

                                                                let current_peer = match dynamic_tabs.tab_index_to_content(active_tab) {
                                                                    TabContent::Direct(id) => Some(id),
                                                                    _ => None,
                                                                };
                                                                let is_current_dm = current_peer.as_ref() == Some(&peer_id_str);

                                                                let sender_display = peer_display_name(&peer_id_str, &local_nicknames, &received_nicknames);
                                                                let dm_msgs = dm_messages.entry(peer_id_str.clone()).or_default();
                                                                let msg = format!("{} {} [{}] {}", ts, latency, sender_display, content);
                                                                dm_msgs.push_back(msg.clone());
                                                                if dm_msgs.len() > MAX_MESSAGES {
                                                                    dm_msgs.pop_front();
                                                                }

                                                                if !is_current_dm {
                                                                    *unread_dms.entry(peer_id_str.clone()).or_insert(0) += 1;
                                                                    dynamic_tabs.add_dm_tab(peer_id_str.clone());
                                                                } else {
                                                                    log_debug(&logs, format!("Received DM from {}: {}", &short_peer_id(&peer_id_str), content));
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
                                                                    nickname: Some(own_nickname.clone()),
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
                                                    Libp2pSwarmEvent::Behaviour(AppEv::Mdns(
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
                                                    Libp2pSwarmEvent::Behaviour(AppEv::Mdns(
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
                                                    Libp2pSwarmEvent::NewListenAddr { address, .. } => {
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
                                                    Libp2pSwarmEvent::ConnectionEstablished { peer_id, .. } => {
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
                                                    Libp2pSwarmEvent::ConnectionClosed {
                                                        peer_id, cause: _, ..
                                                    } => {
                                                        concurrent_peers = concurrent_peers.saturating_sub(1);
                                                        log_debug(&logs, format!("Concurrent peers: {} (disconnected: {})", concurrent_peers, short_peer_id(&peer_id.to_string())));
                                                        if let Err(e) = save_peer_session(concurrent_peers as i32) {
                                                            log_debug(&logs, format!("Failed to save peer session: {}", e));
                                                        }
                                                    }
                                                    Libp2pSwarmEvent::Dialing { peer_id: Some(_pid), .. } => {}
                                                    Libp2pSwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                                                        log_debug(&logs, format!("Dial failed: peer={:?}, error={}", peer_id, error));
                                                    }
                                                    Libp2pSwarmEvent::ExpiredListenAddr { address, .. } => {
                                                        log_debug(&logs, format!("Expired listen addr: {}", address));
                                                    }
                                                    Libp2pSwarmEvent::ListenerError { listener_id, error } => {
                                                        log_debug(&logs, format!("Listener {:?} error: {}", listener_id, error));
                                                    }
                                                    Libp2pSwarmEvent::ListenerClosed { listener_id, reason, addresses } => {
                                                        log_debug(&logs, format!("Listener {:?} closed: {:?} ({:?})", listener_id, reason, addresses));
                                                    }
                                                    Libp2pSwarmEvent::IncomingConnection { .. } => {}
                                                    Libp2pSwarmEvent::IncomingConnectionError { connection_id, local_addr, send_back_addr, error, .. } => {
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
                                                                let peers_start_row = 3;
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
                                                                let notification_rows = if unread_broadcasts > 0 || !unread_dms.is_empty() { 1 } else { 0 };
                                                                let content_start_row = 3 + notification_rows as u16;

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
                                                                exit_tui()?;
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
                                                                            let dm_input = dm_inputs.entry(peer_id.clone()).or_insert_with(init_textarea);
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
                                                                        let topic = gossipsub::IdentTopic::new("test-net");

                                                                        if let Some(system_msg) = handle_chat_input(&text, &mut swarm, &topic, &own_nickname, &topic_str, &logs) {
                                                                            messages.push_back((system_msg, None));
                                                                        } else if !text.trim().is_empty() {
                                                                            if let Some(msg) = handle_send_broadcast(&text, &mut swarm, &topic, &own_nickname, &topic_str, &logs) {
                                                                                messages.push_back((msg, None));
                                                                            }
                                                                        }
                                                                        chat_input = init_textarea();
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
                                                                        let dm_input = dm_inputs.entry(target.clone()).or_insert_with(init_textarea);
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
                                                                                    if let Some(ta) = dm_inputs.get_mut(&target) {
                                                                                        *ta = init_textarea();
                                                                                    }
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
                                                                                nickname: Some(own_nickname.clone()),
                                                                            };

                                                                            swarm.behaviour_mut().request_response.send_request(&peer_id, dm);
                                                                            log_debug(&logs, format!("DM request sent to {}", target));

                                                                            if let Err(e) = save_message(&text, None, &topic_str, true, Some(&peer_id.to_string())) {
                                                                                log_debug(&logs, format!("Failed to save DM: {}", e));
                                                                            }
                                                                        }
                                                                        if let Some(ta) = dm_inputs.get_mut(&target) {
                                                                            *ta = init_textarea();
                                                                        }
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
                                                                    KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                                                        if !peers.is_empty() {
                                                                            let idx = peer_selection.min(peers.len() - 1);
                                                                            if let Some((peer_id, _, _)) = peers.get(idx).cloned() {
                                                                                let input = match dm_inputs.entry(peer_id.clone()) {
                                                                                    std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                                                                                    std::collections::hash_map::Entry::Vacant(e) => {
                                                                                        let mut ta = TextArea::default();
                                                                                        ta.set_line_number_style(Style::default());
                                                                                        ta.set_cursor_line_style(Style::default());
                                                                                        e.insert(ta)
                                                                                    }
                                                                                };
                                                                                input.input(Input::from(crossterm::event::KeyEvent::new(
                                                                                    KeyCode::Char('/'), KeyModifiers::NONE
                                                                                )));
                                                                                input.input(Input::from(crossterm::event::KeyEvent::new(
                                                                                    KeyCode::Char('s'), KeyModifiers::NONE
                                                                                )));
                                                                                input.input(Input::from(crossterm::event::KeyEvent::new(
                                                                                    KeyCode::Char('e'), KeyModifiers::NONE
                                                                                )));
                                                                                input.input(Input::from(crossterm::event::KeyEvent::new(
                                                                                    KeyCode::Char('t'), KeyModifiers::NONE
                                                                                )));
                                                                                input.input(Input::from(crossterm::event::KeyEvent::new(
                                                                                    KeyCode::Char('n'), KeyModifiers::NONE
                                                                                )));
                                                                                input.input(Input::from(crossterm::event::KeyEvent::new(
                                                                                    KeyCode::Char('a'), KeyModifiers::NONE
                                                                                )));
                                                                                input.input(Input::from(crossterm::event::KeyEvent::new(
                                                                                    KeyCode::Char('m'), KeyModifiers::NONE
                                                                                )));
                                                                                input.input(Input::from(crossterm::event::KeyEvent::new(
                                                                                    KeyCode::Char('e'), KeyModifiers::NONE
                                                                                )));
                                                                                input.input(Input::from(crossterm::event::KeyEvent::new(
                                                                                    KeyCode::Char(' '), KeyModifiers::NONE
                                                                                )));
                                                                                active_tab = dynamic_tabs.add_dm_tab(peer_id.clone());
                                                                            }
                                                                        }
                                                                    }
                                                                    KeyCode::Up if !peers.is_empty() => {
                                                                        peer_selection = peer_selection.saturating_sub(1);
                                                                    }
                                                                    KeyCode::Down if !peers.is_empty() => {
                                                                        peer_selection = (peer_selection + 1).min(peers.len() - 1);
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
                                    chat_scroll_offset = auto_scroll_offset(total, visible_height);
                                } else {
                                    chat_scroll_offset = chat_scroll_offset.min(total.saturating_sub(1));
                                }

                                let chat_title = scroll_title("Messages", chat_scroll_offset, total);
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

                                let log_title = scroll_title("Log", debug_scroll_offset, total);
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
            Err(_e) => {
                color_eyre::eyre::bail!("Failed to create terminal. Use bin/p2p_chat for CLI mode");
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct RunningState {
        pub messages: VecDeque<(String, Option<String>)>,
        pub dm_messages: HashMap<String, VecDeque<String>>,
        pub peers: VecDeque<(String, String, String)>,
        pub dynamic_tabs: DynamicTabs,
        pub active_tab: usize,
        pub dm_inputs: HashMap<String, TextArea<'static>>,
        pub chat_input: TextArea<'static>,
        pub concurrent_peers: usize,
        pub peer_selection: usize,
        pub debug_scroll_offset: usize,
        pub debug_auto_scroll: bool,
        pub chat_scroll_offset: usize,
        pub chat_auto_scroll: bool,
        pub own_nickname: String,
        pub local_nicknames: HashMap<String, String>,
        pub received_nicknames: HashMap<String, String>,
        pub unread_broadcasts: u32,
        pub unread_dms: HashMap<String, u32>,
        pub topic_str: String,
        pub logs: Arc<Mutex<VecDeque<String>>>,
    }

    impl RunningState {
        pub fn new(
            topic_str: String,
            logs: Arc<Mutex<VecDeque<String>>>,
            own_nickname: String,
            local_nicknames: HashMap<String, String>,
            received_nicknames: HashMap<String, String>,
            initial_messages: VecDeque<(String, Option<String>)>,
            initial_peers: VecDeque<(String, String, String)>,
        ) -> Self {
            Self {
                messages: initial_messages,
                dm_messages: HashMap::new(),
                peers: initial_peers,
                dynamic_tabs: DynamicTabs::new(),
                active_tab: 0,
                dm_inputs: HashMap::new(),
                chat_input: TextArea::default(),
                concurrent_peers: 0,
                peer_selection: 0,
                debug_scroll_offset: 0,
                debug_auto_scroll: true,
                chat_scroll_offset: 0,
                chat_auto_scroll: true,
                own_nickname,
                local_nicknames,
                received_nicknames,
                unread_broadcasts: 0,
                unread_dms: HashMap::new(),
                topic_str,
                logs,
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
    p2p_app::logging::set_tui_log_callback(move |msg| {
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

    // Use new 4-task architecture instead of monolithic tokio::select!
    tui::main_loop::run_new_tui(swarm, "test-net".to_string(), logs).await
}
