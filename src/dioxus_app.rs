use crate::dioxus_styles::STYLESHEET;
use crate::{DisplayMessage, PeerRecord, SwarmCommand, SwarmEvent};
use dioxus::prelude::*;
use futures_channel::mpsc::UnboundedSender;
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

pub static SWARM_CMD_TX: OnceLock<Mutex<tokio::sync::mpsc::Sender<SwarmCommand>>> = OnceLock::new();
pub static SWARM_EVENT_TX: OnceLock<UnboundedSender<SwarmEvent>> = OnceLock::new();

pub struct InitData {
    pub own_nickname: String,
    pub local_peer_id: String,
    pub topic_str: String,
    pub local_nicknames: HashMap<String, String>,
    pub received_nicknames: HashMap<String, String>,
    pub self_nicknames_for_peers: HashMap<String, String>,
    pub messages: VecDeque<DisplayMessage>,
    pub message_ids: VecDeque<Option<String>>,
    pub sent_at: HashMap<String, f64>,
    pub peers: VecDeque<PeerRecord>,
    pub broadcast_receipts: HashMap<String, HashMap<String, f64>>,
    pub dm_receipts: HashMap<String, (String, f64)>,
}

pub static INIT_DATA: OnceLock<Mutex<Option<InitData>>> = OnceLock::new();

const MAX_MESSAGE_HISTORY: usize = 1000;
const MAX_DM_HISTORY: usize = 1000;

struct AppState {
    messages: VecDeque<DisplayMessage>,
    message_ids: VecDeque<Option<String>>,
    broadcast_receipts: HashMap<String, HashMap<String, f64>>,
    sent_at_by_msg_id: HashMap<String, f64>,
    dm_messages: HashMap<String, VecDeque<String>>,
    dm_message_ids: HashMap<String, VecDeque<Option<String>>>,
    dm_receipts: HashMap<String, (String, f64)>,
    peers: VecDeque<PeerRecord>,
    concurrent_peers: usize,
    local_nicknames: HashMap<String, String>,
    received_nicknames: HashMap<String, String>,
    self_nicknames_for_peers: HashMap<String, String>,
    active_tab: usize,
    dm_tabs: Vec<String>,
    chat_input: String,
    dm_input: HashMap<String, String>,
    own_nickname: String,
    local_peer_id: String,
    topic_str: String,
    editing_nickname: bool,
    editing_nickname_peer: Option<String>,
    popup: Option<String>,
    connected: bool,
    logs: VecDeque<String>,
}

fn send_swarm_cmd(cmd: SwarmCommand) {
    if let Some(tx) = SWARM_CMD_TX.get()
        && let Ok(tx) = tx.lock()
    {
        let _ = tx.try_send(cmd);
    }
}

fn send_chat(state: &mut Signal<AppState>) {
    let input = { state.read().chat_input.trim().to_string() };
    if input.is_empty() {
        return;
    }
    let msg_id = crate::gen_msg_id();
    let ts = crate::format_system_time(SystemTime::now());
    let nickname = { state.read().own_nickname.clone() };
    let display = format!("{} [{}] {}", ts, nickname, input);
    state.write().messages.push_back(DisplayMessage {
        text: display,
        sender_peer_id: None,
    });
    state.write().message_ids.push_back(Some(msg_id.clone()));
    if state.read().messages.len() > MAX_MESSAGE_HISTORY {
        state.write().messages.pop_front();
        state.write().message_ids.pop_front();
    }
    send_swarm_cmd(SwarmCommand::Publish {
        content: input,
        nickname: Some(nickname),
        msg_id: Some(msg_id),
    });
    state.write().chat_input.clear();
}

fn send_dm(state: &mut Signal<AppState>, peer_id: &str) {
    let pid = peer_id.to_string();
    let input = {
        let s = state.read();
        s.dm_input
            .get(&pid)
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    if input.is_empty() {
        return;
    }
    let msg_id = crate::gen_msg_id();
    let ts = crate::format_system_time(SystemTime::now());
    let nickname = { state.read().own_nickname.clone() };
    let display = format!("{} [{}] {}", ts, nickname, input);
    state
        .write()
        .dm_messages
        .entry(pid.clone())
        .or_default()
        .push_back(display);
    state
        .write()
        .dm_message_ids
        .entry(pid.clone())
        .or_default()
        .push_back(Some(msg_id.clone()));
    send_swarm_cmd(SwarmCommand::SendDm {
        peer_id: pid.clone(),
        content: input,
        nickname: Some(nickname),
        msg_id: Some(msg_id),
        ack_for: None,
    });
    let msgs_len = state
        .read()
        .dm_messages
        .get(&pid)
        .map(|m| m.len())
        .unwrap_or(0);
    if msgs_len > MAX_DM_HISTORY {
        state.write().dm_messages.get_mut(&pid).unwrap().pop_front();
        state
            .write()
            .dm_message_ids
            .get_mut(&pid)
            .unwrap()
            .pop_front();
    }
    state.write().dm_input.insert(pid, String::new());
}

fn process_swarm_event(state: &mut Signal<AppState>, event: SwarmEvent) {
    match event {
        SwarmEvent::BroadcastMessage(e) => {
            let mut s = state.write();
            if let Some(n) = e.nickname.as_ref() {
                s.received_nicknames.insert(e.peer_id.clone(), n.clone());
                let _ = crate::set_peer_received_nickname(&e.peer_id, n);
            }
            if e.content.trim().is_empty() && e.nickname.is_some() {
                return;
            }
            let ts = crate::format_system_time(SystemTime::now());
            let sender =
                crate::peer_display_name(&e.peer_id, &s.local_nicknames, &s.received_nicknames);
            let msg = format!(
                "{} {} [{}] {}",
                ts,
                e.latency.unwrap_or_default(),
                sender,
                e.content
            );
            s.messages.push_back(DisplayMessage {
                text: msg,
                sender_peer_id: Some(e.peer_id.clone()),
            });
            s.message_ids.push_back(e.msg_id);
            if s.messages.len() > MAX_MESSAGE_HISTORY {
                s.messages.pop_front();
                s.message_ids.pop_front();
            }
            let _ = crate::save_message(&e.content, Some(&e.peer_id), &s.topic_str, false, None);
        }
        SwarmEvent::DirectMessage(e) => {
            let mut s = state.write();
            if let Some(n) = e.nickname.as_ref() {
                s.received_nicknames.insert(e.peer_id.clone(), n.clone());
                let _ = crate::set_peer_received_nickname(&e.peer_id, n);
            }
            if e.content.trim().is_empty() && e.nickname.is_some() {
                return;
            }
            let ts = crate::format_system_time(SystemTime::now());
            let sender =
                crate::peer_display_name(&e.peer_id, &s.local_nicknames, &s.received_nicknames);
            let msg = format!(
                "{} {} [{}] {}",
                ts,
                e.latency.unwrap_or_default(),
                sender,
                e.content
            );
            s.dm_message_ids
                .entry(e.peer_id.clone())
                .or_default()
                .push_back(e.msg_id);
            let msgs = s.dm_messages.entry(e.peer_id.clone()).or_default();
            msgs.push_back(msg);
            if msgs.len() > MAX_DM_HISTORY {
                msgs.pop_front();
                s.dm_message_ids.get_mut(&e.peer_id).unwrap().pop_front();
            }
            if !s.dm_tabs.contains(&e.peer_id) {
                s.dm_tabs.push(e.peer_id.clone());
            }
            let _ = crate::save_message(
                &e.content,
                Some(&e.peer_id),
                &s.topic_str,
                true,
                Some(&e.peer_id),
            );
        }
        SwarmEvent::Receipt {
            peer_id,
            ack_for,
            received_at: _,
        } => {
            let mut s = state.write();
            let at = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            let mut is_dm = false;
            for ids in s.dm_message_ids.values() {
                if ids
                    .iter()
                    .any(|id| id.as_ref().is_some_and(|v| v == &ack_for))
                {
                    is_dm = true;
                    break;
                }
            }
            if is_dm {
                s.dm_receipts.insert(ack_for.clone(), (peer_id.clone(), at));
                let _ = crate::save_receipt(&ack_for, &peer_id, 1, at);
            }
            s.broadcast_receipts
                .entry(ack_for.clone())
                .or_default()
                .insert(peer_id.clone(), at);
            let _ = crate::save_receipt(&ack_for, &peer_id, 0, at);
        }
        SwarmEvent::PeerConnected(peer_id) => {
            let mut s = state.write();
            s.concurrent_peers += 1;
            if !s.peers.iter().any(|p| p.peer_id == peer_id)
                && let Ok(peer) = crate::save_peer(&peer_id, &[])
            {
                let fs = crate::format_peer_datetime(peer.first_seen);
                let ls = crate::format_peer_datetime(peer.last_seen);
                s.peers.push_back(PeerRecord {
                    peer_id: peer_id.clone(),
                    first_seen: fs,
                    last_seen: ls,
                });
            }
            let nickname = s.own_nickname.clone();
            let msg_id = crate::gen_msg_id();
            send_swarm_cmd(SwarmCommand::SendDm {
                peer_id,
                content: String::new(),
                nickname: Some(nickname),
                msg_id: Some(msg_id),
                ack_for: None,
            });
        }
        SwarmEvent::PeerDisconnected(peer_id) => {
            let mut s = state.write();
            s.concurrent_peers = s.concurrent_peers.saturating_sub(1);
            s.logs.push_back(format!("Peer disconnected: {}", peer_id));
            if s.logs.len() > 500 {
                s.logs.pop_front();
            }
        }
        SwarmEvent::ListenAddrEstablished(addr) => {
            let mut s = state.write();
            s.logs.push_back(format!("Listening on: {}", addr));
            if s.logs.len() > 500 {
                s.logs.pop_front();
            }
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered { peer_id, .. } => {
            let mut s = state.write();
            if !s.peers.iter().any(|p| p.peer_id == peer_id) {
                let now = crate::now_timestamp();
                s.peers.push_back(PeerRecord {
                    peer_id,
                    first_seen: now.clone(),
                    last_seen: now,
                });
            }
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerExpired { peer_id } => {
            let mut s = state.write();
            s.logs.push_back(format!("Peer expired: {}", peer_id));
            if s.logs.len() > 500 {
                s.logs.pop_front();
            }
        }
    }
}

fn initialize_from_init_data(state: &mut Signal<AppState>) {
    if let Some(mutex) = INIT_DATA.get()
        && let Ok(mut guard) = mutex.lock()
        && let Some(data) = guard.take()
    {
        let mut s = state.write();
        s.own_nickname = data.own_nickname;
        s.local_peer_id = data.local_peer_id;
        s.topic_str = data.topic_str;
        s.local_nicknames = data.local_nicknames;
        s.received_nicknames = data.received_nicknames;
        s.self_nicknames_for_peers = data.self_nicknames_for_peers;
        s.messages = data.messages;
        s.message_ids = data.message_ids;
        s.sent_at_by_msg_id = data.sent_at;
        s.peers = data.peers;
        s.broadcast_receipts = data.broadcast_receipts;
        s.dm_receipts = data.dm_receipts;
    }
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    let mut state = use_signal(|| AppState {
        messages: VecDeque::new(),
        message_ids: VecDeque::new(),
        broadcast_receipts: HashMap::new(),
        sent_at_by_msg_id: HashMap::new(),
        dm_messages: HashMap::new(),
        dm_message_ids: HashMap::new(),
        dm_receipts: HashMap::new(),
        peers: VecDeque::new(),
        concurrent_peers: 0,
        local_nicknames: HashMap::new(),
        received_nicknames: HashMap::new(),
        self_nicknames_for_peers: HashMap::new(),
        active_tab: 0,
        dm_tabs: Vec::new(),
        chat_input: String::new(),
        dm_input: HashMap::new(),
        own_nickname: String::new(),
        local_peer_id: String::new(),
        topic_str: String::new(),
        editing_nickname: false,
        editing_nickname_peer: None,
        popup: None,
        connected: true,
        logs: VecDeque::new(),
    });

    let mut init_state = state;
    use_hook(move || {
        initialize_from_init_data(&mut init_state);
    });

    let mut coroutine_state = state;
    let _coroutine = use_coroutine(
        move |mut rx: futures_channel::mpsc::UnboundedReceiver<SwarmEvent>| async move {
            while let Ok(event) = rx.recv().await {
                process_swarm_event(&mut coroutine_state, event);
            }
        },
    );

    if SWARM_EVENT_TX.get().is_none() {
        let sender = _coroutine.tx();
        let _ = SWARM_EVENT_TX.set(sender);
    }

    let s = state.read();
    let active_tab = s.active_tab;
    let dm_tabs = s.dm_tabs.clone();
    let connected = s.connected;
    let peer_count = s.concurrent_peers;
    let nickname = s.own_nickname.clone();
    let peer_id = s.local_peer_id.clone();
    let chat_input = s.chat_input.clone();
    let messages = s.messages.clone();
    let bc_receipts = s.broadcast_receipts.clone();
    let msg_ids = s.message_ids.clone();
    let _sent_at = s.sent_at_by_msg_id.clone();
    let peers = s.peers.clone();
    let local_nicks = s.local_nicknames.clone();
    let received_nicks = s.received_nicknames.clone();
    let editing = s.editing_nickname;
    let editing_peer = s.editing_nickname_peer.clone();
    let popup_text = s.popup.clone();
    let logs = s.logs.clone();
    let status_label = if connected {
        "Connected"
    } else {
        "Disconnected"
    };
    let status_class = if connected {
        "status online"
    } else {
        "status offline"
    };
    let edit_short = editing_peer
        .clone()
        .map(|p| crate::short_peer_id(&p))
        .unwrap_or_default();
    let edit_current_nick = editing_peer
        .clone()
        .and_then(|p| local_nicks.get(&p).cloned())
        .unwrap_or_default();
    let editing_modal = if editing {
        Some(rsx! {
            div { class: "modal-overlay", onclick: move |_| { state.write().editing_nickname = false; state.write().editing_nickname_peer = None; },
                div { class: "modal", onclick: move |e| e.stop_propagation(),
                    h3 { "Edit Nickname for {edit_short}" }
                    input {
                        class: "input-field",
                        value: "{edit_current_nick}",
                        oninput: move |e| { state.write().local_nicknames.insert(editing_peer.clone().unwrap_or_default(), e.value()); },
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                let peer = state.read().editing_nickname_peer.clone().unwrap_or_default();
                                let nick = state.read().local_nicknames.get(&peer).cloned().unwrap_or_default();
                                let _ = crate::set_peer_local_nickname(&peer, &nick);
                                state.write().editing_nickname = false;
                                state.write().editing_nickname_peer = None;
                            }
                        },
                    }
                    div { class: "modal-buttons",
                        button {
                            onclick: move |_| {
                                let peer = state.read().editing_nickname_peer.clone().unwrap_or_default();
                                let nick = state.read().local_nicknames.get(&peer).cloned().unwrap_or_default();
                                let _ = crate::set_peer_local_nickname(&peer, &nick);
                                state.write().editing_nickname = false;
                                state.write().editing_nickname_peer = None;
                            },
                            "Save"
                        }
                        button { onclick: move |_| { state.write().editing_nickname = false; state.write().editing_nickname_peer = None; }, "Cancel" }
                    }
                }
            }
        })
    } else {
        None
    };
    let popup_modal = popup_text.as_ref().map(|text| {
        rsx! {
            div { class: "modal-overlay", onclick: move |_| state.write().popup = None,
                div { class: "modal", onclick: move |e| e.stop_propagation(),
                    pre { "{text}" }
                    div { class: "modal-buttons",
                        button { onclick: move |_| state.write().popup = None, "Close" }
                    }
                }
            }
        }
    });
    drop(s);

    rsx! {
        {editing_modal}
        {popup_modal}
        style { {STYLESHEET} }
        div { class: "app",
            div { class: "header",
                h1 { "P2P Chat" }
                span { class: "{status_class}", "{status_label}" }
                span { class: "peer-count", "Peers: {peer_count}" }
                span { class: "nickname", "[{nickname}]" }
                span { class: "peer-id", "{crate::short_peer_id(&peer_id)}" }
            }
            div { class: "tab-bar",
                {
                    let mut tabs = vec!["Chat".to_string(), "Peers".to_string(), "Log".to_string()];
                    for dt in &dm_tabs {
                        tabs.push(format!("DM: {}", crate::short_peer_id(dt)));
                    }
                    tabs.into_iter().enumerate().map(|(i, title)| {
                        let is_active = i == active_tab;
                        let tab_class = if is_active { "tab active" } else { "tab" };
                        rsx! {
                            button {
                                class: "{tab_class}",
                                onclick: move |_| state.write().active_tab = i,
                                "{title}"
                            }
                        }
                    })
                }
            }
            div { class: "content",
                {
                    if active_tab == 0 {
                        rsx! {
                            div { class: "messages chat-messages",
                                {
                                    messages.iter().enumerate().map(|(i, dm)| {
                                        let receipt_info = msg_ids.get(i)
                                            .and_then(|id| id.as_ref())
                                            .and_then(|id| bc_receipts.get(id))
                                            .map(|rcpts| format!(" ({} receipts)", rcpts.len()))
                                            .unwrap_or_default();
                                        rsx! {
                                            div { class: "message", key: "{i}",
                                                span { "{dm.text}" }
                                                span { class: "receipt-info", "{receipt_info}" }
                                            }
                                        }
                                    })
                                }
                            }
                            div { class: "message-input",
                                input {
                                    class: "input-field",
                                    value: "{chat_input}",
                                    oninput: move |e| state.write().chat_input = e.value(),
                                    onkeydown: move |e| {
                                        if e.key() == Key::Enter && !e.data().modifiers().shift() {
                                            send_chat(&mut state);
                                        }
                                    },
                                }
                                button { class: "send-btn", onclick: move |_| send_chat(&mut state), "Send" }
                            }
                        }
                    } else if active_tab == 1 {
                        rsx! {
                            div { class: "peers-view",
                                h2 { "Peers ({peer_count} connected)" }
                                div { class: "peer-list",
                                    div { class: "peer-item peer-header",
                                        span { "Peer ID" }
                                        span { "Nickname" }
                                        span { "Actions" }
                                    }
                                    {
                                        peers.iter().map(|p| {
                                            let display_name = crate::peer_display_name(&p.peer_id, &local_nicks, &received_nicks);
                                            let short = crate::short_peer_id(&p.peer_id);
                                            let pid_clone = p.peer_id.clone();
                                            rsx! {
                                                div { class: "peer-item", key: "{pid}",
                                                    span { class: "peer-id", title: "{pid}", "{short}" }
                                                    span { class: "peer-nickname", "{display_name}" }
                                                    span { class: "peer-actions",
                                                        button {
                                                            onclick: move |_| {
                                                                state.write().editing_nickname = true;
                                                                state.write().editing_nickname_peer = Some(pid_clone.clone());
                                                            },
                                                            "Edit Nickname"
                                                        }
                                                    }
                                                }
                                            }
                                        })
                                    }
                                }
                            }
                        }
                    } else if active_tab >= 2 && active_tab - 2 < dm_tabs.len() {
                        let dm_peer = dm_tabs[active_tab - 2].clone();
                        let dm_msgs = state.read().dm_messages.get(&dm_peer).cloned().unwrap_or_default();
                        let dm_ids = state.read().dm_message_ids.get(&dm_peer).cloned().unwrap_or_default();
                        let dm_rcpts = state.read().dm_receipts.clone();
                        let dm_input_val = state.read().dm_input.get(&dm_peer).cloned().unwrap_or_default();
                        let short = crate::short_peer_id(&dm_peer);
                        let dm_peer_input = dm_peer.clone();
                        let dm_peer_keydown = dm_peer.clone();
                        let dm_peer_btn = dm_peer.clone();
                        rsx! {
                            div { class: "messages dm-messages",
                                h3 { "DM with {short}" }
                                {
                                    dm_msgs.iter().enumerate().map(|(i, msg)| {
                                        let rcp = dm_ids.get(i)
                                            .and_then(|id| id.as_ref())
                                            .and_then(|id| dm_rcpts.get(id))
                                            .map(|(_, at)| {
                                                let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs_f64();
                                                let lat = now - at;
                                                if lat < 1.0 { format!(" read ~{:.0}ms ago", lat * 1000.0) }
                                                else { format!(" read ~{:.1}s ago", lat) }
                                            })
                                            .unwrap_or_default();
                                        rsx! {
                                            div { class: "message", key: "{i}",
                                                span { "{msg}" }
                                                span { class: "receipt-info", "{rcp}" }
                                            }
                                        }
                                    })
                                }
                            }
                            div { class: "message-input",
                                input {
                                    class: "input-field",
                                    value: "{dm_input_val}",
                                    oninput: move |e| { state.write().dm_input.insert(dm_peer_input.clone(), e.value()); },
                                    onkeydown: move |e| {
                                        if e.key() == Key::Enter && !e.data().modifiers().shift() {
                                            send_dm(&mut state, &dm_peer_keydown);
                                        }
                                    },
                                }
                                button { class: "send-btn", onclick: move |_| send_dm(&mut state, &dm_peer_btn), "Send" }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "log-view",
                                {
                                    logs.iter().map(|log| {
                                        rsx! {
                                            div { class: "log-entry", "{log}" }
                                        }
                                    })
                                }
                            }
                        }
                }
            }

        }
    }
    }
}
