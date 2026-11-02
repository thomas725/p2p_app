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
use std::collections::VecDeque;
use std::io::Stdout;
use tokio::sync::mpsc;

/// The popup to render: the device-binding prompt wins while one is active, then
/// any live call overlay, then the ad-hoc `state.popup` text.
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
        p2p_app::CallState::InCall => Some(format!(
            "In call with {peer}\n\
             \nHang up with Esc"
        )),
        p2p_app::CallState::Ended => None,
    }
}

/// Convert `AppState` to `TuiRenderState` for library rendering
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

/// Convert `AppState` to `TuiRenderState` for library rendering
fn app_state_to_render_state(state: &AppState) -> p2p_app::TuiRenderState {
    let tab_titles = state.dynamic_tabs.all_titles();
    let (messages, message_peer_ids) = message_texts(&state.messages);
    let (last_connection_lost, last_connection_peer) = last_disconnection(state);
    let peers = render_peer_records(&state.peers);

    p2p_app::TuiRenderState {
        tab_titles,
        active_tab: state.active_tab,
        messages,
        message_peer_ids,
        message_ids: state.message_ids.clone(),
        broadcast_receipts: state.broadcast_receipts.clone(),
        peers,
        broadcast_sent_to_peer: state.broadcast_sent_to_peer.clone(),
        contacts: state.contacts.clone(),
        expanded_contact_ids: state.expanded_contact_ids.clone(),
        connected_peer_ids: state
            .connected
            .connected_peer_ids()
            .map(str::to_owned)
            .collect(),
        dm_messages: state
            .dm_messages
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        dm_message_ids: state
            .dm_message_ids
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        dm_receipts: state.dm_receipts.clone(),
        input_text: state.chat_input.lines().join("\n"),
        log_messages: get_tui_logs().into_iter().collect(),
        input_mode: state.input_mode,
        nickname_peer_id: state.editing_nickname_peer.clone().unwrap_or_default(),
        peer_count: state.connected.len(),
        mouse_capture: state.env.mouse_capture,
        kitty_keyboard_active: state.env.kitty_keyboard_active,
        popup: render_popup_text(state),
        chat_scroll_offset: state.chat_scroll_offset,
        auto_scroll: state.auto_scroll,
        chat_unread_count: state.chat_unread_count,
        log_scroll_offset: state.log_scroll_offset,

        dm_scroll_state: state
            .dm_scroll_state
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect(),
        dm_broadcast_scroll_state: state
            .dm_broadcast_scroll_state
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect(),
        group_summaries: state.group_summaries.clone(),
        group_messages: state
            .group_messages
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        group_message_ids: state
            .group_message_ids
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        group_message_peer_ids: state
            .group_message_peer_ids
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        group_scroll_state: state
            .group_scroll_state
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect(),
        group_selection: state.group_selection,

        peer_selection: state.peer_selection,
        peer_sort_column: state.peer_sort_column,
        peer_sort_ascending: state.peer_sort_ascending,
        own_nickname: state.own_nickname.clone(),
        local_peer_id: state.local_peer_id.clone(),
        db_url: state.db_url.clone(),
        platform: state.platform.clone(),
        network_size: state.network_size.clone(),
        listen_addrs: state.listen_addrs.clone(),
        network: p2p_app::net_diag::snapshot(),
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
    match &tab_content {
        TabContent::Chat => {
            tui_render::render_chat_content(
                f,
                chunks.get(1).copied().unwrap_or_default(),
                &mut render_state,
            );
        }
        TabContent::Contacts => {
            tui_render::render_peers_content(
                f,
                chunks.get(1).copied().unwrap_or_default(),
                &render_state,
            );
        }
        TabContent::Direct(peer_id) => {
            tui_render::render_dm_content(
                f,
                chunks.get(1).copied().unwrap_or_default(),
                peer_id,
                &mut render_state,
            );
        }
        TabContent::Log => {
            tui_render::render_log_content(
                f,
                chunks.get(1).copied().unwrap_or_default(),
                &render_state,
            );
        }
        TabContent::Settings => {
            tui_render::render_settings_content(
                f,
                chunks.get(1).copied().unwrap_or_default(),
                &render_state,
            );
        }
        TabContent::PeerInfo(peer_id) => {
            tui_render::render_peer_info_content(
                f,
                chunks.get(1).copied().unwrap_or_default(),
                peer_id,
                &render_state,
            );
        }
        TabContent::Groups => {
            tui_render::render_groups_content(
                f,
                chunks.get(1).copied().unwrap_or_default(),
                &render_state,
            );
        }
        TabContent::GroupChat(group_id) => {
            tui_render::render_group_chat_content(
                f,
                chunks.get(1).copied().unwrap_or_default(),
                group_id,
                &render_state,
            );
        }
    }

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
        render_popup(f, text.clone());
    }
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
        }
    })
}

#[cfg(test)]
#[path = "../../../../tests/unit/unit_bin_tui_render_loop_mod.rs"]
mod tests;
