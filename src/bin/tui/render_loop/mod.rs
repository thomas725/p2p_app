mod layout;

use super::main_loop::RenderEvent;
use super::state::{AppState, SharedState};
use p2p_app::get_tui_logs;
use p2p_app::tui_render;
use p2p_app::tui_render::render_popup;
use p2p_app::tui_tabs::TabContent;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::io::{Stdout, Write};
use tokio::sync::mpsc;

/// The popup to render, highest precedence first: device-binding intro, group
/// leave, message deletion, live call overlay, then ad-hoc `state.popup` text.
fn render_popup_text(state: &AppState) -> Option<String> {
    state
        .device_binding_prompt
        .as_ref()
        .map(|p| {
            format!(
                "Device-binding intro from {} (pubkey {} bytes)\n\
                 \nAccept with 'a' \u{b7} decline with 'd' or Esc",
                p2p_app::fmt::peer_id_suffix(&p.peer_id),
                p.device_pubkey.len()
            )
        })
        .or_else(|| {
            state.group_leave_prompt.as_ref().map(|p| {
                format!(
                    "Leave group \"{}\"?\n\
                     \nThis deletes the group and its history from this device. \
                     Other members keep their copy.\n\
                     \nConfirm with 'y' or Enter \u{b7} cancel with Esc",
                    p.display_name
                )
            })
        })
        .or_else(|| {
            state.delete_message_prompt.as_ref().map(|p| {
                format!(
                    "Delete this sent message from this device?\n\
                     \n{}\n\
                     \nOnly this device loses the message; other peers keep their copy.\n\
                     \nConfirm with 'y' or Enter \u{b7} cancel with Esc",
                    p.preview
                )
            })
        })
        .or_else(|| call_overlay_text(state))
        .or_else(|| state.popup.clone())
}

/// The call-overlay text for a live session (`None` when no call is active or
/// the session has settled). Accept/decline/cancel/hang-up keys are handled by
/// the input processor; this is the visual prompt mirroring the device-binding
/// one.
fn call_overlay_text(state: &AppState) -> Option<String> {
    let session = state.call_session.as_ref()?;
    if !session.is_active() {
        return None;
    }
    let peer = p2p_app::peer_display_name(
        &session.peer_id,
        &state.local_nicknames,
        &state.received_nicknames,
    );
    match session.state {
        p2p_app::CallState::Ringing => Some(format!(
            "Incoming call from {peer}\n\
             \nAccept with 'a' \u{b7} decline with any other key"
        )),
        p2p_app::CallState::Calling => Some(format!(
            "Calling {peer} \u{2026}\n\
             \nCancel with 'c' or Esc"
        )),
        p2p_app::CallState::InCall => Some(state.call_media.audio_error.as_deref().map_or_else(
            || {
                format!(
                    "In call with {peer}\n\
                     \nMic: {} \u{b7} peer frames: {}\n\
                     'm' toggles mute \u{b7} Esc hangs up",
                    if state.call_media.muted {
                        "muted"
                    } else {
                        "on"
                    },
                    state.call_media.frames_received
                )
            },
            |error| {
                format!(
                    "In call with {peer}\n\
                     \nAudio unavailable: {error}\n\
                     Esc hangs up"
                )
            },
        )),
        p2p_app::CallState::Ended => None,
    }
}

/// Project the broadcast-chat `DisplayMessage` history into the text + sender
/// columns the render layer consumes.
fn message_texts(
    messages: &VecDeque<p2p_app::DisplayMessage>,
) -> (VecDeque<String>, VecDeque<Option<String>>) {
    (
        messages.iter().map(|dm| dm.text.clone()).collect(),
        messages
            .iter()
            .map(|dm| dm.sender_peer_id.clone())
            .collect(),
    )
}

/// Project the peer ring into the `PeerRecord` rows the render layer consumes.
fn render_peer_records(peers: &VecDeque<p2p_app::PeerRecord>) -> Vec<p2p_app::PeerRecord> {
    peers.iter().cloned().collect()
}

/// Resolve the connected-overlay's last disconnection into the (time, peer)
/// pair the render layer shows, if any.
fn last_disconnection(state: &AppState) -> (Option<f64>, Option<String>) {
    match state.connected.last_disconnection() {
        Some((peer, at)) => (Some(at), Some(peer.to_string())),
        None => (None, None),
    }
}

/// Clone a keyed chat-log map so the render layer can outlive the state borrow.
fn cloned_log<V: Clone>(log: &HashMap<String, V>) -> BTreeMap<String, V> {
    log.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

/// Convert `AppState` to `TuiRenderState` for library rendering.
///
/// `pub(super)` so the Settings mouse handler can hit-test the exact same
/// visual rows the renderer draws.
pub(super) fn app_state_to_render_state(state: &AppState) -> p2p_app::TuiRenderState {
    let tab_titles = state.dynamic_tabs.all_titles();
    let active_tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    let reply_preview = state
        .reply_draft(&active_tab_content)
        .map(p2p_app::reply::quoted_header);
    let (messages, message_peer_ids) = message_texts(&state.messages);
    let (last_connection_lost, last_connection_peer) = last_disconnection(state);
    let peers = render_peer_records(&state.peers);
    let connected_peer_count = state.connected.len();
    let network_size = p2p_app::network::network_size_label(
        i64::try_from(connected_peer_count).unwrap_or(i64::MAX),
    );

    p2p_app::TuiRenderState {
        tab_titles,
        active_tab: state.active_tab,
        messages,
        message_peer_ids,
        message_ids: state.message_ids.clone(),
        broadcast_receipts: state.broadcast_receipts.clone(),
        reactions: state.reactions.clone(),
        peers,
        broadcast_sent_to_peer: state.broadcast_sent_to_peer.clone(),
        contacts: state.contacts.clone(),
        contact_search_total: (!state.contact_search.is_empty())
            .then_some(state.contacts_all.len()),
        expanded_contact_ids: state.expanded_contact_ids.clone(),
        blocked_peers: state.blocked_peers.clone(),
        connected_peer_ids: state
            .connected
            .connected_peer_ids()
            .map(str::to_owned)
            .collect(),
        dm_messages: cloned_log(&state.dm_messages),
        dm_message_ids: cloned_log(&state.dm_message_ids),
        dm_receipts: state.dm_receipts.clone(),
        typing_peer_ids: state.typing_peers.keys().cloned().collect(),
        input_text: state.chat_input.lines().join("\n"),
        log_messages: get_tui_logs().into_iter().collect(),
        input_mode: state.input_mode,
        nickname_peer_id: state.editing_nickname_peer.clone().unwrap_or_default(),
        peer_count: state.connected.len(),
        mouse_capture: state.env.mouse_capture,
        kitty_keyboard_active: state.env.kitty_keyboard_active,
        popup: render_popup_text(state),
        reply_preview,
        chat_scroll_offset: state.chat_scroll_offset,
        auto_scroll: state.auto_scroll,
        chat_unread_count: state.chat_unread_count,
        log_scroll_offset: state.log_scroll_offset,

        dm_scroll_state: cloned_log(&state.dm_scroll_state),
        dm_unread_count: cloned_log(&state.dm_unread_count),
        dm_broadcast_scroll_state: cloned_log(&state.dm_broadcast_scroll_state),
        group_summaries: state.group_summaries.clone(),
        group_messages: cloned_log(&state.group_messages),
        group_message_ids: cloned_log(&state.group_message_ids),
        group_message_peer_ids: cloned_log(&state.group_message_peer_ids),
        group_scroll_state: cloned_log(&state.group_scroll_state),
        group_unread_count: cloned_log(&state.group_unread_count),
        group_selection: state.group_selection,
        shares: state.shares.clone(),
        share_grants: state.share_grants.clone(),
        share_selection: state.share_selection,
        share_event_notice: state.share_event_notice.clone(),

        peer_selection: state.peer_selection,
        peer_sort_column: state.peer_sort_column,
        peer_sort_ascending: state.peer_sort_ascending,
        own_nickname: state.own_nickname.clone(),
        local_peer_id: state.local_peer_id.clone(),
        db_url: state.db_url.clone(),
        platform: state.platform.clone(),
        network_size,
        network: p2p_app::net_diag::snapshot(),
        network_peers: state.network_peers.clone(),
        palette: state.palette,
        network_peer_selection: state.network_peer_selection,
        network_peer_kind: state.network_peer_kind.map(|kind| kind.as_str().to_owned()),
        last_connection_lost,
        last_connection_peer,
        runtime: p2p_app::tui_render_state::RuntimeStatus {
            connected: true,
            node_running: true,
        },
        own_user_id: state.own_user_id.clone(),
        own_user_devices: state.own_user_devices.clone(),
        local_nicknames: state.local_nicknames.clone(),
        received_nicknames: state.received_nicknames.clone(),
        peer_table_offset: state.peer_table_offset,
    }
}

/// Render the active tab's content pane. Split out of [`render_frame`] so the
/// per-tab dispatch does not push the frame orchestration over clippy's line
/// budget.
fn render_active_tab(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    tab_content: &TabContent,
    render_state: &mut p2p_app::TuiRenderState,
) {
    match tab_content {
        TabContent::Chat => {
            tui_render::render_chat_content(f, area, render_state);
        }
        TabContent::Contacts => {
            tui_render::render_peers_content(f, area, render_state);
        }
        TabContent::Direct(peer_id) => {
            tui_render::render_dm_content(f, area, peer_id, render_state);
        }
        TabContent::Log => {
            tui_render::render_log_content(f, area, render_state);
        }
        TabContent::Settings => {
            tui_render::render_settings_content(f, area, render_state);
        }
        TabContent::PeerInfo(peer_id) => {
            tui_render::render_peer_info_content(f, area, peer_id, render_state);
        }
        TabContent::Groups => {
            tui_render::render_groups_content(f, area, render_state);
        }
        TabContent::GroupChat(group_id) => {
            tui_render::render_group_chat_content(f, area, group_id, render_state);
        }
        TabContent::Shares => {
            tui_render::render_shares_content(f, area, render_state);
        }
    }
}

/// Orchestrate the frame layout and dispatch to appropriate tab renderers
fn render_frame(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tabs
            Constraint::Min(0),    // messages
            Constraint::Length(5), // input area
            Constraint::Length(1), // shortcuts
            Constraint::Length(1), // status
        ])
        .split(f.area());

    let mut render_state = app_state_to_render_state(state);

    tui_render::render_tabs(
        f,
        chunks.first().copied().unwrap_or_default(),
        &render_state,
    );

    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    render_active_tab(
        f,
        chunks.get(1).copied().unwrap_or_default(),
        &tab_content,
        &mut render_state,
    );

    layout::render_input_section(
        f,
        chunks.get(2).copied().unwrap_or_default(),
        state,
        &tab_content,
    );
    tui_render::render_shortcuts(
        f,
        chunks.get(3).copied().unwrap_or_default(),
        &tab_content,
        state.env.kitty_keyboard_active,
    );
    layout::render_status_bar(f, chunks.get(4).copied().unwrap_or_default(), state);

    // The popup/overlay layer: paint the merged text (device-binding prompt or
    // call overlay wins over the ad-hoc `AppState.popup`), which is what
    // `render_popup_text` folded into `render_state.popup`.
    if let Some(ref text) = render_state.popup {
        render_popup(f, text.clone(), render_state.palette);
    }
}

/// Render one frame with the real binary renderer for key-session tests.
///
/// The library renderer resolves tabs from legacy titles; production uses the
/// typed `DynamicTabs` lookup. Sessions must observe exactly what a terminal
/// user sees, so tests go through this wrapper rather than the library path.
#[cfg(test)]
pub(super) fn render_frame_for_test(f: &mut Frame, state: &AppState) {
    render_frame(f, state);
}

/// Spawns the render loop task that renders only when requested
pub fn spawn_render_loop(
    state: SharedState,
    mut terminal: Terminal<CrosstermBackend<Stdout>>,
    mut render_rx: mpsc::Receiver<RenderEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while render_rx.recv().await.is_some() {
            let mut s = state.lock().await;
            if let Some(error) = s.call_audio.take_error() {
                s.call_media.audio_error = Some(error);
            }
            let _ = terminal.draw(|f| {
                s.terminal_width = usize::from(f.area().width);
                s.chat_area_height = usize::from(f.area().height.saturating_sub(10));
                // On the first frame that shows the Contacts tab, resolve every
                // known peer's display name once so the library's name cache
                // is warm and switching sort columns never pays a per-peer DB
                // lookup.
                if !s.env.peer_names_warmed
                    && matches!(
                        s.dynamic_tabs.tab_index_to_content(s.active_tab),
                        TabContent::Contacts
                    )
                {
                    for p in &s.peers {
                        let _ = p2p_app::get_peer_display_name(&p.peer_id);
                    }
                    s.env.peer_names_warmed = true;
                }
                render_frame(f, &s);
            });

            // Drain notifications and clipboard payloads queued by the command
            // processor *after* the frame is drawn so a bell or OSC sequence
            // can never interleave with ratatui's screen writes. Both queues
            // are taken before dropping the state lock, and a single locked
            // stdout handle is reused for the whole drain.
            let pending = std::mem::take(&mut s.pending_notifications);
            let clipboard = std::mem::take(&mut s.pending_clipboard);
            drop(s);
            if !pending.is_empty() || !clipboard.is_empty() {
                let mut stdout = std::io::stdout().lock();
                for notification in &pending {
                    let sequence = p2p_app::notifications::terminal_sequence(notification);
                    if !sequence.is_empty() {
                        let _ = stdout.write_all(sequence.as_bytes());
                    }
                }
                for content in &clipboard {
                    let sequence = p2p_app::clipboard::copy_sequence(content);
                    let _ = stdout.write_all(sequence.as_bytes());
                }
                let _ = stdout.flush();
            }
        }
    })
}

#[cfg(test)]
#[path = "../../../../tests/unit/unit_bin_tui_render_loop_mod.rs"]
mod tests;
