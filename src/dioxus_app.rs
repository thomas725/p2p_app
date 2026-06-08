use crate::dioxus_styles::STYLESHEET;
use crate::dioxus_swarm::process_swarm_event;
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

pub(crate) const MAX_MESSAGE_HISTORY: usize = 1000;
pub(crate) const MAX_DM_HISTORY: usize = 1000;

pub(crate) struct AppState {
    pub(crate) messages: VecDeque<DisplayMessage>,
    pub(crate) message_ids: VecDeque<Option<String>>,
    pub(crate) broadcast_receipts: HashMap<String, HashMap<String, f64>>,
    pub(crate) sent_at_by_msg_id: HashMap<String, f64>,
    pub(crate) dm_messages: HashMap<String, VecDeque<String>>,
    pub(crate) dm_message_ids: HashMap<String, VecDeque<Option<String>>>,
    pub(crate) dm_receipts: HashMap<String, (String, f64)>,
    pub(crate) peers: VecDeque<PeerRecord>,
    pub(crate) concurrent_peers: usize,
    pub(crate) local_nicknames: HashMap<String, String>,
    pub(crate) received_nicknames: HashMap<String, String>,
    pub(crate) self_nicknames_for_peers: HashMap<String, String>,
    pub(crate) active_tab: usize,
    pub(crate) dm_tabs: Vec<String>,
    pub(crate) chat_input: String,
    pub(crate) dm_input: HashMap<String, String>,
    pub(crate) own_nickname: String,
    pub(crate) local_peer_id: String,
    pub(crate) topic_str: String,
    pub(crate) editing_nickname: bool,
    pub(crate) editing_nickname_peer: Option<String>,
    pub(crate) popup: Option<String>,
    pub(crate) connected: bool,
    pub(crate) logs: VecDeque<String>,
}

pub(crate) fn send_swarm_cmd(cmd: SwarmCommand) {
    if let Some(tx) = SWARM_CMD_TX.get()
        && let Ok(tx) = tx.lock()
    {
        let _ = tx.try_send(cmd);
    }
}

fn send_message(state: &mut Signal<AppState>, input: String, is_dm: Option<&str>) {
    let msg_id = crate::gen_msg_id();
    let ts = crate::format_system_time(SystemTime::now());
    let nickname = { state.read().own_nickname.clone() };
    let display = format!("{} [{}] {}", ts, nickname, input);
    let cmd = if let Some(peer_id) = is_dm {
        let pid = peer_id.to_string();
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
        state.write().dm_input.insert(pid.clone(), String::new());
        SwarmCommand::SendDm {
            peer_id: pid,
            content: input,
            nickname: Some(nickname),
            msg_id: Some(msg_id),
            ack_for: None,
        }
    } else {
        state.write().messages.push_back(DisplayMessage {
            text: display,
            sender_peer_id: None,
        });
        state.write().message_ids.push_back(Some(msg_id.clone()));
        if state.read().messages.len() > MAX_MESSAGE_HISTORY {
            state.write().messages.pop_front();
            state.write().message_ids.pop_front();
        }
        state.write().chat_input.clear();
        SwarmCommand::Publish {
            content: input,
            nickname: Some(nickname),
            msg_id: Some(msg_id),
        }
    };
    send_swarm_cmd(cmd);
}

fn send_chat(state: &mut Signal<AppState>) {
    let input = { state.read().chat_input.trim().to_string() };
    if !input.is_empty() {
        send_message(state, input, None);
    }
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
    if !input.is_empty() {
        send_message(state, input, Some(&pid));
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

fn tab_class(i: usize, active_tab: usize) -> &'static str {
    if i == active_tab { "tab active" } else { "tab" }
}

fn render_chat_tab(
    mut state: Signal<AppState>,
    messages: VecDeque<DisplayMessage>,
    msg_ids: VecDeque<Option<String>>,
    bc_receipts: HashMap<String, HashMap<String, f64>>,
    chat_input: String,
) -> Element {
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
}

fn render_peers_tab(
    mut state: Signal<AppState>,
    peers: VecDeque<PeerRecord>,
    local_nicks: HashMap<String, String>,
    received_nicks: HashMap<String, String>,
    peer_count: usize,
) -> Element {
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
                            div { class: "peer-item", key: "{p.peer_id}",
                                span { class: "peer-id", title: "{p.peer_id}", "{short}" }
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
}

fn render_dm_tab(mut state: Signal<AppState>, dm_peer: String) -> Element {
    let dm_msgs = state
        .read()
        .dm_messages
        .get(&dm_peer)
        .cloned()
        .unwrap_or_default();
    let dm_ids = state
        .read()
        .dm_message_ids
        .get(&dm_peer)
        .cloned()
        .unwrap_or_default();
    let dm_rcpts = state.read().dm_receipts.clone();
    let dm_input_val = state
        .read()
        .dm_input
        .get(&dm_peer)
        .cloned()
        .unwrap_or_default();
    drop(state.read());
    let short = crate::short_peer_id(&dm_peer);
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
                oninput: move |e| { state.write().dm_input.insert(dm_peer.clone(), e.value()); },
                onkeydown: move |e| {
                    if e.key() == Key::Enter && !e.data().modifiers().shift() {
                        send_dm(&mut state, &dm_peer_keydown);
                    }
                },
            }
            button { class: "send-btn", onclick: move |_| send_dm(&mut state, &dm_peer_btn), "Send" }
        }
    }
}

fn render_log_tab(logs: VecDeque<String>) -> Element {
    rsx! {
        div { class: "log-view",
            { logs.iter().map(|log| rsx! { div { class: "log-entry", "{log}" } }) }
        }
    }
}

fn render_edit_modal(
    mut state: Signal<AppState>,
    editing: bool,
    edit_short: String,
    edit_current_nick: String,
) -> Option<Element> {
    if !editing {
        return None;
    }
    Some(rsx! {
        div { class: "modal-overlay", onclick: move |_| { state.write().editing_nickname = false; state.write().editing_nickname_peer = None; },
            div { class: "modal", onclick: move |e| e.stop_propagation(),
                h3 { "Edit Nickname for {edit_short}" }
                input {
                    class: "input-field",
                    value: "{edit_current_nick}",
                    oninput: move |e| {
                            let peer = state.read().editing_nickname_peer.clone().unwrap_or_default();
                            state.write().local_nicknames.insert(peer, e.value());
                        },
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
}

fn render_popup_modal(mut state: Signal<AppState>, popup_text: Option<String>) -> Option<Element> {
    popup_text.as_ref().map(|text| {
        let text = text.clone();
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
    })
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    let state = use_signal(|| AppState {
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
    let editing = s.editing_nickname;
    let editing_peer = s.editing_nickname_peer.clone();
    let popup_text = s.popup.clone();
    let edit_short = editing_peer
        .clone()
        .map(|p| crate::short_peer_id(&p))
        .unwrap_or_default();
    let edit_current_nick = editing_peer
        .clone()
        .and_then(|p| s.local_nicknames.get(&p).cloned())
        .unwrap_or_default();
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
    drop(s);

    rsx! {
        {render_edit_modal(state, editing, edit_short, edit_current_nick)}
        {render_popup_modal(state, popup_text)}
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
                        let cls = tab_class(i, active_tab);
                        let mut st = state;
                        rsx! {
                            button {
                                class: "{cls}",
                                onclick: move |_| st.write().active_tab = i,
                                "{title}"
                            }
                        }
                    })
                }
            }
            div { class: "content",
                {
                    if active_tab == 0 {
                        let s = state.read();
                        let messages = s.messages.clone();
                        let msg_ids = s.message_ids.clone();
                        let bc_receipts = s.broadcast_receipts.clone();
                        let chat_input = s.chat_input.clone();
                        drop(s);
                        render_chat_tab(state, messages, msg_ids, bc_receipts, chat_input)
                    } else if active_tab == 1 {
                        let s = state.read();
                        let peers = s.peers.clone();
                        let local_nicks = s.local_nicknames.clone();
                        let received_nicks = s.received_nicknames.clone();
                        let peer_count = s.concurrent_peers;
                        drop(s);
                        render_peers_tab(state, peers, local_nicks, received_nicks, peer_count)
                    } else if active_tab >= 2 && active_tab - 2 < dm_tabs.len() {
                        let dm_peer = dm_tabs[active_tab - 2].clone();
                        render_dm_tab(state, dm_peer)
                    } else {
                        let logs = state.read().logs.clone();
                        render_log_tab(logs)
                    }
                }
            }
        }
    }
}
