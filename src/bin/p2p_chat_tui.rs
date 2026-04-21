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
