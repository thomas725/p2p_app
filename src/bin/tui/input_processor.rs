use super::event_source::InputEvent;
use super::main_loop::RenderEvent;
use super::state::SharedState;
use p2p_app::{SwarmCommand, p2plog_debug};
use tokio::sync::mpsc;

use crate::tui::click_handlers::{handle_mouse_left_click, load_dm_messages};
use crate::tui::scroll_handlers::{handle_mouse_scroll, handle_navigation_key, handle_scroll_key};

fn update_dm_transcript_labels(
    dm_messages: &mut std::collections::HashMap<String, std::collections::VecDeque<String>>,
    peer_id: &str,
    old_nick: &str,
    new_nick: &str,
) {
    if let Some(dm_msgs) = dm_messages.get_mut(peer_id) {
        p2p_app::tui_helpers::relabel_dm_transcript(dm_msgs, old_nick, new_nick);
    }
}

/// Pure: flip the mouse capture flag. Returns the new state.
const fn flip_mouse_capture_state(state: &mut super::state::AppState) -> bool {
    state.env.mouse_capture = !state.env.mouse_capture;
    state.env.mouse_capture
}

/// Pure: dismiss the popup if one is open. Returns true if a popup was dismissed.
fn dismiss_popup(state: &mut super::state::AppState) -> bool {
    if state.popup.is_some() {
        state.popup = None;
        true
    } else {
        false
    }
}

/// Pure: extract nickname update data from state. Returns None if the new nickname is empty.
fn prepare_nickname_update(
    state: &super::state::AppState,
) -> Option<(String, String, Option<String>)> {
    let new_nickname = state.chat_input.lines().join("\n");
    if new_nickname.trim().is_empty() {
        return None;
    }
    let (old_nickname, peer_id) = state.editing_nickname_peer.as_ref().map_or_else(
        || (state.own_nickname.clone(), None),
        |pid| {
            let old = state
                .self_nicknames_for_peers
                .get(pid)
                .cloned()
                .unwrap_or_else(|| state.own_nickname.clone());
            (old, Some(pid.clone()))
        },
    );
    Some((new_nickname, old_nickname, peer_id))
}

/// Toggles mouse capture mode (F12)
fn toggle_mouse_capture(state: &mut super::state::AppState) {
    use ratatui::crossterm::execute;
    flip_mouse_capture_state(state);
    let mode = if state.env.mouse_capture {
        "enabled"
    } else {
        "disabled"
    };
    p2plog_debug(format!("Mouse capture {mode}"));
    let mut stdout = std::io::stdout();
    let _ = if state.env.mouse_capture {
        execute!(stdout, crossterm::event::EnableMouseCapture)
    } else {
        execute!(stdout, crossterm::event::DisableMouseCapture)
    };
}

async fn handle_nickname_submission(
    state: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) {
    let Some((new_nickname, old_nickname, peer_id)) = prepare_nickname_update(state) else {
        state.cancel_nickname_edit();
        return;
    };

    if let Some(peer_id) = peer_id {
        state
            .self_nicknames_for_peers
            .insert(peer_id.clone(), new_nickname.clone());
        let _ = p2p_app::set_peer_self_nickname_for_peer(&peer_id, &new_nickname);
        let _ = swarm_cmd_tx
            .send(SwarmCommand::SendDm {
                peer_id: peer_id.clone(),
                content: String::new(),
                nickname: Some(new_nickname.clone()),
                msg_id: None,
                ack_for: None,
                attachment: None,
            })
            .await;
        update_dm_transcript_labels(
            &mut state.dm_messages,
            &peer_id,
            &old_nickname,
            &new_nickname,
        );
        p2plog_debug(format!("Updated per-peer nickname to: {new_nickname}"));
    } else {
        state.own_nickname.clone_from(&new_nickname);
        let _ = p2p_app::set_self_nickname(&new_nickname);
        for p in &state.peers {
            let peer_id = &p.peer_id;
            if state.self_nicknames_for_peers.contains_key(peer_id) {
                continue;
            }
            let _ = swarm_cmd_tx
                .send(SwarmCommand::SendDm {
                    peer_id: peer_id.clone(),
                    content: String::new(),
                    nickname: Some(new_nickname.clone()),
                    msg_id: None,
                    ack_for: None,
                    attachment: None,
                })
                .await;
        }
        let peer_ids: Vec<String> = state.dm_messages.keys().cloned().collect();
        for peer_id in peer_ids {
            if state.self_nicknames_for_peers.contains_key(&peer_id) {
                continue;
            }
            update_dm_transcript_labels(
                &mut state.dm_messages,
                &peer_id,
                &old_nickname,
                &new_nickname,
            );
        }
        p2plog_debug(format!("Updated broadcast nickname to: {new_nickname}"));
    }
    state.cancel_nickname_edit();
}

/// Creates or joins a public group by name: persists it, subscribes to its
/// topic, and opens its chat tab.
async fn create_or_join_group(
    state: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    name: &str,
) {
    if name.trim().is_empty() {
        state.cancel_group_create();
        return;
    }
    match p2p_app::groups::create_public_group(name.trim()) {
        Ok(group) => {
            let group_id = group.group_id.clone();
            let display_name = group.display_name.clone();
            let _ = swarm_cmd_tx
                .send(SwarmCommand::SubscribeGroup {
                    group_id: group_id.clone(),
                })
                .await;
            state.reload_group_summaries();
            state.group_selection = state
                .group_summaries
                .iter()
                .position(|g| g.group.group_id == group_id)
                .unwrap_or(0);
            super::click_handlers::open_group_chat(state, &group_id, &display_name);
            state.input_mode = p2p_app::InputMode::Chat;
            state.chat_input = super::TextArea::default();
            p2plog_debug(format!("Created/joined group: {display_name} ({group_id})"));
        }
        Err(e) => p2plog_debug(format!("Failed to create/join group: {e}")),
    }
}

/// Groups-tab keys: `g` opens the create/join prompt, `d` arms the leave
/// confirmation for the selected group (`y`/Enter confirms). Both are no-ops
/// while a text-input mode is active, which the caller guards.
fn handle_groups_key(c: char, s: &mut super::state::AppState) {
    match c {
        'g' => {
            s.input_mode = p2p_app::InputMode::CreatingGroup;
            s.chat_input = super::TextArea::default();
            p2plog_debug("Started group create/join".to_string());
        }
        'd' => {
            if let Some(group) = s.group_summaries.get(s.group_selection) {
                let group_id = group.group.group_id.clone();
                let display_name = group.group.display_name.clone();
                s.begin_group_leave(group_id, display_name);
                p2plog_debug("Leave group confirmation armed".to_string());
            }
        }
        _ => {}
    }
}

/// Leaves a group after the user confirms: delete the local rows (messages +
/// membership + the group itself) and unsubscribe from its gossipsub topic,
/// close the group's chat tab, and refresh the list. Local-only by design —
/// public groups have no server-side membership to notify, so other members
/// keep their copy of the history (mirrors Flutter's "Leave group").
async fn confirm_group_leave(state: &SharedState, swarm_cmd_tx: &mpsc::Sender<SwarmCommand>) {
    let prompt = {
        let mut s = state.lock().await;
        s.group_leave_prompt.take()
    };
    let Some(prompt) = prompt else {
        return;
    };
    match p2p_app::groups::delete_group(&prompt.group_id) {
        Ok(()) => {
            let _ = swarm_cmd_tx
                .send(SwarmCommand::UnsubscribeGroup {
                    group_id: prompt.group_id.clone(),
                })
                .await;
            state.lock().await.apply_group_leave(&prompt.group_id);
            p2plog_debug(format!(
                "Left group: {} ({})",
                prompt.display_name, prompt.group_id
            ));
        }
        Err(e) => p2plog_debug(format!("Failed to leave group: {e}")),
    }
}

/// Handles Ctrl+W (close DM or Peer Info tab)
fn handle_close_dm_tab(
    state: &mut super::state::AppState,
    tab_content: &p2p_app::tui_tabs::TabContent,
) {
    if let Some(closed_idx) = state.dynamic_tabs.remove_tab(tab_content) {
        state.active_tab = if closed_idx > 0 {
            closed_idx.saturating_sub(1)
        } else {
            0
        };
        state.peer_selection = 0;
        p2plog_debug(format!("Closed tab: {tab_content:?}"));
    }
}

/// Queue a call signal to `peer_id` on the calls lane (invite/accept/reject/
/// end; the session id is echoed by every kind).
async fn send_call_signal(
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    peer_id: &str,
    call_id: &str,
    kind: &str,
) {
    let _ = swarm_cmd_tx
        .send(SwarmCommand::SendCallSignal {
            peer_id: peer_id.to_string(),
            signal: Box::new(p2p_app::CallSignal::new(kind, call_id.to_string())),
        })
        .await;
}

/// Place an outgoing call to `peer_id`: mint a fresh session and send its
/// invite. A no-op while a call is already active (one session at a time).
async fn start_call_to_peer(
    state: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    peer_id: &str,
) {
    if state
        .call_session
        .as_ref()
        .is_some_and(p2p_app::CallSession::is_active)
    {
        p2plog_debug("Call already active; ignoring new ring".to_string());
        return;
    }
    let session = p2p_app::CallSession::outgoing(peer_id);
    let call_id = session.call_id.clone();
    send_call_signal(swarm_cmd_tx, peer_id, &call_id, "invite").await;
    state.call_session = Some(session);
    p2plog_debug(format!("Placing call to {peer_id} ({call_id})"));
}

/// `c` starts a call to the peer targeted by `tab`: the selected
/// contact/device on the Contacts tab, or the Peer Info owner.
async fn start_call_for_tab(
    tab: &p2p_app::tui_tabs::TabContent,
    s: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) {
    if let Some(peer_id) = match tab {
        p2p_app::tui_tabs::TabContent::Contacts => s.selected_contact_peer_id(),
        p2p_app::tui_tabs::TabContent::PeerInfo(pid) => Some(pid.clone()),
        _ => None,
    } {
        start_call_to_peer(s, swarm_cmd_tx, &peer_id).await;
    }
}

/// Route a key to the active call overlay. A live session swallows all input
/// until it resolves (mirrors the device-binding prompt):
///   * `Ringing` — `a`/`A` accepts; any other key (Esc included) declines.
///   * `Calling` — `c`/`C`/Esc cancels the outgoing ring; other keys ignored.
///   * `InCall`  — Esc hangs up; other keys ignored.
///
/// Always consumes the key (the caller short-circuits before generic dispatch).
async fn handle_call_overlay_key(
    key_event: &crossterm::event::KeyEvent,
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    render_tx: &mpsc::Sender<RenderEvent>,
) {
    let s = state.lock().await;
    let Some(session) = s.call_session.as_ref() else {
        return;
    };
    let (call_id, peer_id, call_state) = (
        session.call_id.clone(),
        session.peer_id.clone(),
        session.state,
    );
    drop(s);

    match call_state {
        p2p_app::CallState::Ringing => {
            let accept = matches!(key_event.code, crossterm::event::KeyCode::Char('a' | 'A'));
            let kind = if accept { "accept" } else { "reject" };
            let mut s = state.lock().await;
            if accept {
                if let Some(session) = s.call_session.as_mut() {
                    session.on_signal(&p2p_app::CallSignal::new(kind, call_id.clone()));
                }
            } else {
                s.call_session = None;
            }
            drop(s);
            send_call_signal(swarm_cmd_tx, &peer_id, &call_id, kind).await;
            p2plog_debug(if accept {
                format!("Accepted call {call_id}")
            } else {
                format!("Declined call {call_id} (ringing)")
            });
        }
        p2p_app::CallState::Calling => {
            let is_esc = key_event.code == crossterm::event::KeyCode::Esc;
            let cancel =
                is_esc || matches!(key_event.code, crossterm::event::KeyCode::Char('c' | 'C'));
            if cancel {
                let mut s = state.lock().await;
                s.call_session = None;
                drop(s);
                send_call_signal(swarm_cmd_tx, &peer_id, &call_id, "end").await;
                p2plog_debug(format!("Cancelled outgoing call {call_id}"));
            }
        }
        p2p_app::CallState::InCall => {
            if key_event.code == crossterm::event::KeyCode::Esc {
                let mut s = state.lock().await;
                s.call_session = None;
                drop(s);
                send_call_signal(swarm_cmd_tx, &peer_id, &call_id, "end").await;
                p2plog_debug(format!("Hanging up call {call_id}"));
            }
        }
        p2p_app::CallState::Ended => {}
    }
    let _ = render_tx.send(RenderEvent).await;
}

/// Handles Esc: dismiss popup / cancel nickname edit / cancel group create /
/// return to broadcast chat.
async fn handle_esc_key(state: &SharedState, render_tx: &mpsc::Sender<RenderEvent>) {
    let mut s = state.lock().await;
    dismiss_popup(&mut s);
    decline_device_binding_prompt(&mut s);
    if s.cancel_group_leave() {
        p2plog_debug("Cancelled group leave".to_string());
    } else if s.is_editing_nickname() {
        s.cancel_nickname_edit();
        p2plog_debug("Cancelled nickname edit".to_string());
    } else if s.is_creating_group() {
        s.cancel_group_create();
        p2plog_debug("Cancelled group create/join".to_string());
    } else {
        s.active_tab = 0;
        s.chat_scroll_offset = 0;
        s.auto_scroll.chat = true;
        s.chat_unread_count = 0;
        p2plog_debug("Returned to Broadcast Chat (Esc)".to_string());
    }
    drop(s);
    let _ = render_tx.send(RenderEvent).await;
}

/// Decline the active device-binding prompt (leaves the peer unbound). Also
/// used by the generic popup dismiss and Esc paths so the prompt can never
/// linger after the popup is gone.
pub fn decline_device_binding_prompt(state: &mut super::state::AppState) {
    if state.device_binding_prompt.take().is_some() {
        p2plog_debug("Declined device-binding intro".to_string());
    }
}

/// Accept the active device-binding prompt: mint a signed claim binding the
/// intro's device into this install's user, persist it locally, and send the
/// claim back over the swarm. On mint failure (e.g. the install has no
/// account) the prompt is cleared and the error is surfaced in a popup.
pub async fn accept_device_binding_prompt(
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) {
    let prompt = {
        let mut s = state.lock().await;
        s.device_binding_prompt.take()
    };
    let Some(prompt) = prompt else {
        return;
    };
    match p2p_app::identity::mint_device_binding(&prompt.peer_id, &prompt.device_pubkey) {
        Ok(request) => {
            let _ = swarm_cmd_tx
                .send(SwarmCommand::SendDeviceBinding {
                    peer_id: prompt.peer_id.clone(),
                    request: Box::new(request),
                })
                .await;
            p2plog_debug(format!(
                "Accepted device-binding intro from {}",
                p2p_app::fmt::peer_id_suffix(&prompt.peer_id)
            ));
            // Accepting bound the intro's device into our user: refresh the
            // identity/settings display (device-set grew).
            let mut s = state.lock().await;
            s.refresh_my_user();
        }
        Err(e) => {
            p2plog_debug(format!("Failed to accept device binding: {e}"));
            let mut s = state.lock().await;
            s.popup = Some(format!(
                "Could not accept device binding from {}: {e}",
                p2p_app::fmt::peer_id_suffix(&prompt.peer_id)
            ));
        }
    }
}

/// Opens the Peer Info tab for the selected contact (Contacts tab) or the
/// active DM partner (Direct tab).
fn open_peer_info_for_active_tab(state: &mut super::state::AppState) {
    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    let peer = match &tab_content {
        p2p_app::tui_tabs::TabContent::Contacts => state.selected_contact_peer_id(),
        p2p_app::tui_tabs::TabContent::Direct(pid) => Some(pid.clone()),
        _ => None,
    };
    if let Some(peer_id) = peer {
        state.active_tab = state.dynamic_tabs.add_peer_info_tab(peer_id);
    }
}

/// Sets the peers-table sort column, toggling direction if the same column is
/// chosen again (mirroring Flutter's `_PeerList` header tap behavior), and
/// re-sorts the peer list while preserving the selected peer.
fn apply_peer_sort(state: &mut super::state::AppState, column: usize) {
    if state.peer_sort_column == column {
        state.peer_sort_ascending = !state.peer_sort_ascending;
    } else {
        state.peer_sort_column = column;
        state.peer_sort_ascending = false;
    }
    state.resort_peers();
}

/// Dispatches a peer-list sort key (`1`-`5`, `n`/`m`/`b`/`l`/`f` for columns,
/// `o` to toggle direction) when the Peers tab is active.
fn handle_peer_sort_key(state: &mut super::state::AppState, c: char) {
    match c {
        '1' | 'n' | 'N' => apply_peer_sort(state, 0),
        '2' | 'm' | 'M' => apply_peer_sort(state, 1),
        '3' | 'b' | 'B' => apply_peer_sort(state, 2),
        '4' | 'l' | 'L' => apply_peer_sort(state, 3),
        '5' | 'f' | 'F' => apply_peer_sort(state, 4),
        'o' | 'O' => {
            state.peer_sort_ascending = !state.peer_sort_ascending;
            state.resort_peers();
        }
        _ => {}
    }
}

/// Handles Enter key (send message or multi-line input)
async fn handle_enter_key(
    state: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    shift_held: bool,
    tab_content: p2p_app::tui_tabs::TabContent,
) {
    if shift_held {
        if tab_content.is_input_enabled() {
            state.chat_input.insert_str("\n");
        }
    } else if matches!(tab_content, p2p_app::tui_tabs::TabContent::Contacts) {
        if let Some(peer_id) = state.selected_contact_peer_id() {
            load_dm_messages(state, &peer_id);
            let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id.clone());
            state.active_tab = tab_idx;
            p2plog_debug(format!("Opened DM with peer: {peer_id}"));
        }
    } else if matches!(tab_content, p2p_app::tui_tabs::TabContent::Groups) {
        if state.is_creating_group() {
            let name = state.chat_input.lines().join("\n");
            create_or_join_group(state, swarm_cmd_tx, &name).await;
        } else if let Some(group) = state.group_summaries.get(state.group_selection) {
            let group_id = group.group.group_id.clone();
            let display_name = group.group.display_name.clone();
            super::click_handlers::open_group_chat(state, &group_id, &display_name);
            p2plog_debug(format!("Opened group: {display_name}"));
        }
    } else if let p2p_app::tui_tabs::TabContent::PeerInfo(peer_id) = &tab_content {
        load_dm_messages(state, peer_id);
        let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id.clone());
        state.active_tab = tab_idx;
        p2plog_debug(format!("Opened DM with peer: {peer_id}"));
    } else if tab_content.is_input_enabled() {
        let text: String = state.chat_input.lines().join("\n");
        let trimmed = text.trim_start();
        if trimmed.starts_with("/file ") {
            match parse_file_command(&text) {
                Some(path) => {
                    super::message_handlers::send_file_message(
                        state,
                        swarm_cmd_tx,
                        &path,
                        tab_content,
                    )
                    .await;
                }
                None => state.popup = Some("Usage: /file <path>".to_string()),
            }
        } else if !text.trim().is_empty() {
            super::message_handlers::send_message(state, swarm_cmd_tx, text, None, tab_content)
                .await;
        }
    }
}

/// Pure: parse a `/file` chat command into a path (files lane §12). Comments
/// out of any ordinary message; only an actual `/file <path>` with a non-blank
/// path returns `Some`, everything else stays `None` so it passes through the
/// normal send path.
fn parse_file_command(input: &str) -> Option<String> {
    input
        .trim_start()
        .strip_prefix("/file ")
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
}

/// Open the DM partner's Peer Info when a `Ctrl+I` / `Ctrl+P` chord lands on a
/// Direct tab.
///
/// `Ctrl+P` is the cross-terminal binding: `p` is 0x70, so Ctrl+P sends 0x10
/// (`DLE`), a distinct control byte crossterm uniformly reports as
/// `Char('p')`+CONTROL (the `b'\x01'..=b'\x1A'` arm) whether or not the kitty
/// protocol is active. `Ctrl+I` works only on kitty terminals
/// (`WezTerm`), where it arrives as `Char('i')`+CONTROL; on non-kitty terminals
/// Ctrl+I collapses to the bare Tab byte, which always cycles tabs (see
/// [`process_key_event`]). Exactly one chord is the Peer Info binding on either
/// setup.
fn handle_peer_info_control_chord(
    key_event: &crossterm::event::KeyEvent,
    s: &mut super::state::AppState,
) {
    open_peer_info_for_active_tab(s);
    let chord = if key_event.code == crossterm::event::KeyCode::Char('p') {
        "Ctrl+P"
    } else {
        "Ctrl+I"
    };
    p2plog_debug(format!("Opened Peer Info tab ({chord})"));
}

/// Processes keyboard input events, returns the exit signal (true = quit).
///
/// Early-out handling (Esc, Ctrl+Q, the device-binding prompt, popup
/// dismissal) happens here before the key is dispatched; the actual dispatch
/// against the active tab lives in [`handle_key_dispatch`].
async fn process_key_event(
    key_event: crossterm::event::KeyEvent,
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    render_tx: &mpsc::Sender<RenderEvent>,
) -> bool {
    if key_event
        .modifiers
        .contains(crossterm::event::KeyModifiers::CONTROL)
        && key_event.code == crossterm::event::KeyCode::Char('q')
    {
        p2plog_debug("Exit signal received".to_string());
        return true;
    }

    // A live call session owns the keys until it resolves (mirrors the
    // device-binding prompt): accept/decline a ring, cancel an outgoing call,
    // hang up an active one. Handled before Esc so Esc is a call key here, not
    // the generic dismiss/return-to-chat handling.
    let call_active = {
        let s = state.lock().await;
        s.call_session
            .as_ref()
            .is_some_and(p2p_app::CallSession::is_active)
    };
    if call_active {
        handle_call_overlay_key(&key_event, state, swarm_cmd_tx, render_tx).await;
        return false;
    }

    if key_event.code == crossterm::event::KeyCode::Esc {
        handle_esc_key(state, render_tx).await;
        return false;
    }

    let mut s = state.lock().await;

    // Device-binding intro prompt: 'a'/'A' accepts (mints a signed claim and
    // sends it to the peer), any other key declines. Explicit-accept only —
    // never auto-mint, mirroring the mobile Allow/Decline dialog.
    if s.device_binding_prompt.is_some() {
        let accept = matches!(key_event.code, crossterm::event::KeyCode::Char('a' | 'A'));
        drop(s);
        if accept {
            accept_device_binding_prompt(state, swarm_cmd_tx).await;
        } else {
            let mut s = state.lock().await;
            decline_device_binding_prompt(&mut s);
            drop(s);
        }
        let _ = render_tx.send(RenderEvent).await;
        return false;
    }

    // Group-leave confirmation: 'y'/Enter confirms (delete + unsubscribe),
    // any other key cancels. Single-slot and wins over all other input, like
    // the device-binding prompt.
    if s.group_leave_prompt.is_some() {
        let confirm = matches!(
            key_event.code,
            crossterm::event::KeyCode::Char('y' | 'Y') | crossterm::event::KeyCode::Enter
        );
        drop(s);
        if confirm {
            confirm_group_leave(state, swarm_cmd_tx).await;
        } else {
            let mut s = state.lock().await;
            if s.cancel_group_leave() {
                p2plog_debug("Cancelled group leave".to_string());
            }
            drop(s);
        }
        let _ = render_tx.send(RenderEvent).await;
        return false;
    }

    // Diagnostic: log exactly what crossterm parsed for every input key that
    // touches the Tab/Ctrl+I/Ctrl+P routing, regardless of the kitty flag, so
    // we can tell from the TUI log what the app actually received.
    if matches!(
        key_event.code,
        crossterm::event::KeyCode::Tab
            | crossterm::event::KeyCode::BackTab
            | crossterm::event::KeyCode::Char('i' | 'I' | 'p' | 'P')
    ) {
        p2plog_debug(format!(
            "Key: code={:?} mods={:?}",
            key_event.code, key_event.modifiers
        ));
    }

    // If a popup is open, any key dismisses it (except we still honor exit keys above).
    if dismiss_popup(&mut s) {
        drop(s);
        let _ = render_tx.send(RenderEvent).await;
        return false;
    }

    handle_key_dispatch(key_event, &mut s, swarm_cmd_tx).await;
    drop(s);
    let _ = render_tx.send(RenderEvent).await;
    false
}

/// Dispatch a fully-unlocked key event against the active tab.
///
/// Tab lookups and modifier flags are computed once up front; shared
/// terminal-key semantics (Ctrl+I / Ctrl+P vs bare Tab) live with
/// [`handle_peer_info_control_chord`].
async fn handle_key_dispatch(
    key_event: crossterm::event::KeyEvent,
    s: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) {
    let is_ctrl = key_event
        .modifiers
        .contains(crossterm::event::KeyModifiers::CONTROL);
    let shift_held = key_event
        .modifiers
        .contains(crossterm::event::KeyModifiers::SHIFT);
    let tab = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    let on_contacts = matches!(&tab, p2p_app::tui_tabs::TabContent::Contacts);
    let on_direct = matches!(&tab, p2p_app::tui_tabs::TabContent::Direct(_));
    let on_settings = matches!(&tab, p2p_app::tui_tabs::TabContent::Settings);
    let on_groups = matches!(&tab, p2p_app::tui_tabs::TabContent::Groups);
    let input_enabled = tab.is_input_enabled();

    match key_event.code {
        // Ctrl+I / Ctrl+P both open the DM partner's Peer Info on a Direct tab.
        crossterm::event::KeyCode::Char('i' | 'p')
            if !s.is_editing_nickname() && is_ctrl && on_direct =>
        {
            handle_peer_info_control_chord(&key_event, s);
        }
        // Consume Ctrl+P on non-Direct tabs too (never literal text). Ctrl+I on
        // non-Direct tabs falls through to the chat-input branch, as before.
        crossterm::event::KeyCode::Char('p') if !s.is_editing_nickname() && is_ctrl => {
            p2plog_debug("Peer Info Ctrl+P ignored (no Direct tab)".to_string());
        }
        crossterm::event::KeyCode::BackTab => {
            handle_navigation_key(key_event.code, s).await;
        }
        // Tab carries two meanings: bare Tab cycles tabs on every terminal (a
        // bare 0x09 — Ctrl+I collapses to the same byte on non-kitty terminals,
        // but there Ctrl+P is the distinct Peer Info binding instead, so Tab is
        // always free to cycle). Tab+CONTROL is the CSI-u spelling of Ctrl+I on
        // terminals that send it as an explicitly-modified Tab; it is a no-op.
        crossterm::event::KeyCode::Tab => {
            if is_ctrl {
                p2plog_debug("Ctrl+I (Tab+CONTROL) ignored: no-op on this tab".to_string());
            } else {
                handle_navigation_key(key_event.code, s).await;
            }
        }
        crossterm::event::KeyCode::Up
        | crossterm::event::KeyCode::Down
        | crossterm::event::KeyCode::PageUp
        | crossterm::event::KeyCode::PageDown
        | crossterm::event::KeyCode::Home
        | crossterm::event::KeyCode::End => {
            handle_scroll_key(key_event.code, s).await;
        }
        crossterm::event::KeyCode::F(12) => {
            toggle_mouse_capture(s);
        }
        crossterm::event::KeyCode::Enter => {
            if s.is_editing_nickname() {
                handle_nickname_submission(s, swarm_cmd_tx).await;
            } else {
                handle_enter_key(s, swarm_cmd_tx, shift_held, tab).await;
            }
        }
        crossterm::event::KeyCode::Char('w') if is_ctrl => {
            handle_close_dm_tab(s, &tab);
        }
        crossterm::event::KeyCode::Char('n')
            if !is_ctrl && on_settings && !s.is_editing_nickname() =>
        {
            s.input_mode = p2p_app::InputMode::EditingNickname;
            s.editing_nickname_peer = None;
            s.chat_input = super::TextArea::default();
            p2plog_debug("Started nickname edit from Settings tab".to_string());
        }
        crossterm::event::KeyCode::Char('i')
            if !is_ctrl && on_contacts && !s.is_editing_nickname() =>
        {
            open_peer_info_for_active_tab(s);
            p2plog_debug("Opened Peer Info tab".to_string());
        }
        // `e` expands/collapses the selected contact's device rows (no-op on
        // single-device contacts and device rows).
        crossterm::event::KeyCode::Char('e')
            if !is_ctrl && on_contacts && !s.is_editing_nickname() =>
        {
            s.toggle_expand_selected_contact();
            p2plog_debug("Toggled contact expansion".to_string());
        }
        // `c` places a call to the selected contact/device (Contacts) or the
        // Peer Info owner. Must precede the generic sort-key arm below (which
        // would otherwise swallow the `c`).
        crossterm::event::KeyCode::Char('c')
            if !is_ctrl
                && !s.is_editing_nickname()
                && (on_contacts || matches!(&tab, p2p_app::tui_tabs::TabContent::PeerInfo(_))) =>
        {
            start_call_for_tab(&tab, s, swarm_cmd_tx).await;
        }
        crossterm::event::KeyCode::Char(c)
            if !is_ctrl && on_contacts && !s.is_editing_nickname() =>
        {
            handle_peer_sort_key(s, c);
        }
        crossterm::event::KeyCode::Char(c)
            if !is_ctrl && on_groups && !s.is_editing_nickname() && !s.is_creating_group() =>
        {
            handle_groups_key(c, s);
        }
        _ => {
            if input_enabled || s.is_editing_nickname() || s.is_creating_group() {
                s.chat_input.input(key_event);
            }
        }
    }
}

/// Processes mouse input events
async fn process_mouse_event(
    mouse_event: crossterm::event::MouseEvent,
    state: &SharedState,
    render_tx: &mpsc::Sender<RenderEvent>,
) {
    if matches!(
        mouse_event.kind,
        crossterm::event::MouseEventKind::Moved | crossterm::event::MouseEventKind::Drag(_)
    ) {
        // Track the live hover row so wheel-scroll and click routing on split
        // DM tabs target the pane the pointer is actually over. No frame change
        // results from a move alone, so nothing needs re-rendering.
        state.lock().await.last_mouse_row = mouse_event.row;
        return;
    }

    let mut s = state.lock().await;
    let mut should_render = false;

    s.last_mouse_row = mouse_event.row;

    let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    let is_peers_tab = matches!(tab_content, p2p_app::tui_tabs::TabContent::Contacts);
    let is_scrollable_tab = matches!(
        tab_content,
        p2p_app::tui_tabs::TabContent::Chat
            | p2p_app::tui_tabs::TabContent::Direct(_)
            | p2p_app::tui_tabs::TabContent::GroupChat(_)
            | p2p_app::tui_tabs::TabContent::Log
    );
    let peer_id = if let p2p_app::tui_tabs::TabContent::Direct(pid) = &tab_content {
        Some(pid.as_str())
    } else {
        None
    };

    match mouse_event.kind {
        crossterm::event::MouseEventKind::ScrollUp if is_scrollable_tab => {
            should_render = handle_mouse_scroll(&mut s, "up", peer_id);
        }
        crossterm::event::MouseEventKind::ScrollDown if is_scrollable_tab => {
            should_render = handle_mouse_scroll(&mut s, "down", peer_id);
        }
        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            if dismiss_popup(&mut s) {
                should_render = true;
            } else {
                should_render = handle_mouse_left_click(
                    &mut s,
                    mouse_event.row,
                    mouse_event.column,
                    is_peers_tab,
                );
            }
        }
        _ => {}
    }
    drop(s);
    if should_render {
        let _ = render_tx.send(RenderEvent).await;
    }
}

/// Main input event processor - routes keyboard and mouse events
/// Returns true if exit was requested, false otherwise
pub async fn process_input_event(
    input_event: InputEvent,
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    render_tx: &mpsc::Sender<RenderEvent>,
) -> bool {
    match input_event {
        InputEvent::Key(key_event) => {
            process_key_event(key_event, state, swarm_cmd_tx, render_tx).await
        }
        InputEvent::Mouse(mouse_event) => {
            process_mouse_event(mouse_event, state, render_tx).await;
            false
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_input_processor.rs"]
mod tests;
