use super::event_source::InputEvent;
use super::input_processor::process_input_event;
use super::main_loop::RenderEvent;
use super::state::{AppState, CallMediaStatus, SharedState};
use super::state::{MAX_DM_HISTORY, MAX_MESSAGE_HISTORY, trim_history};
use p2p_app::{DisplayMessage, PeerRecord, SwarmCommand, SwarmEvent, p2plog_debug};
use std::sync::Arc;
use tokio::sync::mpsc;

enum Event {
    Input(InputEvent),
    // Boxed: `SwarmEvent` is much larger than `InputEvent` and the enum is
    // stored on every loop iteration.
    SwarmEvent(Box<SwarmEvent>),
}

#[cfg(test)]
fn sort_peers_by_last_seen(state: &mut AppState) {
    state.peer_selection =
        p2p_app::tui_helpers::sort_peers_by_last_seen(&mut state.peers, state.peer_selection);
}

fn upsert_peer_last_seen(state: &mut AppState, peer_id: &str, seen_at: chrono::NaiveDateTime) {
    let seen_str = p2p_app::format_peer_datetime(seen_at);
    state.peer_selection = p2p_app::tui_helpers::upsert_peer_last_seen(
        &mut state.peers,
        state.peer_selection,
        peer_id,
        &seen_str,
    );
}

/// Pure state mutation for incoming broadcast messages. Returns the formatted message string.
fn apply_broadcast_to_state(
    state: &mut AppState,
    content: &str,
    peer_id: &str,
    latency: Option<&str>,
    msg_id: Option<String>,
    reply_to: Option<&p2p_app::ReplyRef>,
) -> String {
    let ts = p2p_app::format_now();
    let sender_display =
        p2p_app::peer_display_name(peer_id, &state.local_nicknames, &state.received_nicknames);
    let msg = p2p_app::reply::with_quoted_header(
        format!(
            "{ts} {} [{sender_display}] {content}",
            latency.unwrap_or_default(),
        ),
        reply_to,
    );
    state.messages.push_back(DisplayMessage {
        text: msg.clone(),
        sender_peer_id: Some(peer_id.to_string()),
    });
    state.message_ids.push_back(msg_id);
    // Live inbound events have no DB id; the DB loader fills it on restart.
    state.message_db_ids.push_back(None);
    trim_history(&mut state.messages, MAX_MESSAGE_HISTORY);
    trim_history(&mut state.message_ids, MAX_MESSAGE_HISTORY);
    trim_history(&mut state.message_db_ids, MAX_MESSAGE_HISTORY);
    if !state.auto_scroll.chat {
        state.chat_unread_count = state.chat_unread_count.saturating_add(1);
    }
    msg
}

/// Pure state mutation for incoming direct messages. Returns the formatted message string.
fn apply_dm_to_state(
    state: &mut AppState,
    content: &str,
    peer_id: &str,
    latency: Option<&str>,
    msg_id: Option<String>,
    reply_to: Option<&p2p_app::ReplyRef>,
) -> String {
    let ts = p2p_app::format_now();
    let sender_display =
        p2p_app::peer_display_name(peer_id, &state.local_nicknames, &state.received_nicknames);
    let msg = p2p_app::reply::with_quoted_header(
        format!(
            "{ts} {} [{sender_display}] {content}",
            latency.unwrap_or_default(),
        ),
        reply_to,
    );
    state
        .dm_message_ids
        .entry(peer_id.to_string())
        .or_default()
        .push_back(msg_id);
    state
        .dm_message_db_ids
        .entry(peer_id.to_string())
        .or_default()
        .push_back(None);
    let dm_msgs = state.dm_messages.entry(peer_id.to_string()).or_default();
    dm_msgs.push_back(msg.clone());
    trim_history(dm_msgs, MAX_DM_HISTORY);
    if let Some(ids) = state.dm_message_ids.get_mut(peer_id) {
        trim_history(ids, MAX_DM_HISTORY);
    }
    if let Some(ids) = state.dm_message_db_ids.get_mut(peer_id) {
        trim_history(ids, MAX_DM_HISTORY);
    }
    state.dynamic_tabs.add_dm_tab(peer_id.to_string());
    msg
}

/// Pure: check if a receipt is for a DM message
fn is_dm_receipt(state: &AppState, ack_for: &str) -> bool {
    state.dm_message_ids.values().any(|ids| {
        ids.iter()
            .any(|id| id.as_ref().is_some_and(|v| v == ack_for))
    })
}

/// Pure state mutation for receipts
fn apply_receipt_to_state(
    state: &mut AppState,
    ack_for: &str,
    peer_id: &str,
    at: f64,
    is_dm: bool,
) {
    if is_dm {
        state
            .dm_receipts
            .insert(ack_for.to_string(), (peer_id.to_string(), at));
    }
    state
        .broadcast_receipts
        .entry(ack_for.to_string())
        .or_default()
        .insert(peer_id.to_string(), at);
}

/// Pure state mutation for incoming group messages.
///
/// Group messages are persisted centrally by the swarm handler before the
/// event is sent, so the DB is the source of truth for history. Live messages
/// are appended on top only while the group's chat tab is open; a closed chat
/// re-loads from the DB when it is opened (no duplicate re-deliveries).
fn apply_group_message_to_state(
    state: &mut AppState,
    group_id: &str,
    content: &str,
    peer_id: &str,
    msg_id: Option<String>,
    reply_to: Option<&p2p_app::ReplyRef>,
) {
    if state.dynamic_tabs.get_group_tab(group_id).is_none() {
        return;
    }
    let ts = p2p_app::format_now();
    let sender_display =
        p2p_app::peer_display_name(peer_id, &state.local_nicknames, &state.received_nicknames);
    let msg =
        p2p_app::reply::with_quoted_header(format!("{ts} [{sender_display}] {content}"), reply_to);
    let group_msgs = state
        .group_messages
        .entry(group_id.to_string())
        .or_default();
    group_msgs.push_back(msg);
    trim_history(group_msgs, MAX_MESSAGE_HISTORY);
    state
        .group_message_ids
        .entry(group_id.to_string())
        .or_default()
        .push_back(msg_id);
    state
        .group_message_db_ids
        .entry(group_id.to_string())
        .or_default()
        .push_back(None);
    state
        .group_message_peer_ids
        .entry(group_id.to_string())
        .or_default()
        .push_back(Some(peer_id.to_string()));
    if let Some(ids) = state.group_message_ids.get_mut(group_id) {
        trim_history(ids, MAX_MESSAGE_HISTORY);
    }
    if let Some(ids) = state.group_message_db_ids.get_mut(group_id) {
        trim_history(ids, MAX_MESSAGE_HISTORY);
    }
    if let Some(ids) = state.group_message_peer_ids.get_mut(group_id) {
        trim_history(ids, MAX_MESSAGE_HISTORY);
    }
    state.group_scroll_state.insert(group_id.to_string(), {
        let len = group_msgs.len();
        (len, true)
    });
}

/// State mutation: increment connected peer count. Returns new count.
const fn apply_peer_connected_count(state: &mut AppState) -> usize {
    state.concurrent_peers = state.concurrent_peers.saturating_add(1);
    state.concurrent_peers
}

/// State mutation on a peer connection: persist the peer with a fresh last-seen,
/// add-or-refresh the in-memory list record, and mark it as currently connected.
///
/// The last-seen refresh matches Flutter, which reloads peers whenever a peer
/// connects, so a stale DB timestamp is never shown as "last seen" after a
/// reconnect to a peer that is already in the in-memory list.
fn apply_peer_connected_state(state: &mut AppState, peer_id: &str) {
    let now = chrono::Utc::now().naive_utc();
    if let Ok(peer) = p2p_app::save_peer(peer_id, &[]) {
        if state.peers.iter().any(|p| p.peer_id == peer_id) {
            upsert_peer_last_seen(state, peer_id, now);
        } else {
            let first_seen = p2p_app::format_peer_datetime(peer.first_seen);
            let last_seen = p2p_app::format_peer_datetime(peer.last_seen);
            add_peer_to_state_list(state, peer_id, &first_seen, &last_seen);
        }
        state.resort_peers();
    }
    state.connected.on_peer_connected(peer_id.to_string());
}

/// State mutation: add peer to peer list and re-sort.
///
/// This install's own device is never a row: its `peers` record exists only for
/// the device-binding foreign key.
fn add_peer_to_state_list(state: &mut AppState, peer_id: &str, first_seen: &str, last_seen: &str) {
    if peer_id == state.local_peer_id {
        return;
    }
    state.peers.push_back(PeerRecord {
        peer_id: peer_id.to_string(),
        first_seen: first_seen.to_string(),
        last_seen: last_seen.to_string(),
    });
    state.resort_peers();
}

/// State mutation: decrement connected peer count. Returns new count.
const fn apply_peer_disconnected_count(state: &mut AppState) -> usize {
    state.concurrent_peers = state.concurrent_peers.saturating_sub(1);
    state.concurrent_peers
}

#[cfg(feature = "mdns")]
fn apply_peer_discovered_state(state: &mut AppState, peer_id: &str) {
    if !state.peers.iter().any(|p| p.peer_id == peer_id) {
        let now = p2p_app::now_timestamp();
        add_peer_to_state_list(state, peer_id, &now, &now);
    }
}

/// Compose a chat line body from message text plus an attachment chip: text
/// first, chip appended; an attachment-only message renders as just the chip so
/// a file never shows up as a blank row. (Shared with the history-reload paths
/// via `p2p_app::files::render_stored_body`.)
fn render_message_body(content: &str, attachment: Option<&p2p_app::MessageAttachment>) -> String {
    p2p_app::files::render_message_body(content, attachment)
}

/// Build a notification for an incoming user message and queue it for the
/// render loop, unless its conversation is already the active tab (US-028).
/// Rust owns the policy and display text via [`p2p_app::notifications`]; the
/// TUI only decides focus and writes the terminal escape sequence.
fn queue_notification(
    s: &mut AppState,
    event_type: &str,
    peer_id: &str,
    nickname: Option<&str>,
    content: &str,
    group_id: Option<&str>,
) {
    let Some(notification) =
        p2p_app::notifications::for_message(event_type, peer_id, nickname, content, group_id)
    else {
        return;
    };
    if s.active_conversation_key().as_deref() == Some(notification.conversation_key.as_str()) {
        return;
    }
    s.pending_notifications.push(notification);
}

/// Route one inbound chat message into TUI state (and the DB). `is_direct`
/// selects a DM vs a broadcast; an empty-content message that only carries a
/// nickname is treated as a pure nickname update.
fn handle_incoming_message(s: &mut AppState, msg: p2p_app::MessageEvent, is_direct: bool) {
    let p2p_app::MessageEvent {
        content,
        peer_id,
        latency,
        nickname,
        msg_id,
        attachment,
        reply_to,
        encrypted,
    } = msg;
    if let Some(n) = nickname.as_deref() {
        s.received_nicknames.insert(peer_id.clone(), n.to_string());
        let _ = p2p_app::set_peer_received_nickname(&peer_id, n);
    }
    if content.trim().is_empty() && nickname.is_some() && attachment.is_none() && reply_to.is_none()
    {
        upsert_peer_last_seen(s, &peer_id, chrono::Utc::now().naive_utc());
        s.resort_peers();
        return;
    }
    let sender_display =
        p2p_app::peer_display_name(&peer_id, &s.local_nicknames, &s.received_nicknames);
    let body = p2p_app::e2e::mark_encrypted(
        render_message_body(&content, attachment.as_ref()),
        encrypted,
    );
    // Queue the notification while `nickname` is still borrowed — it is moved
    // into `meta` below. The chat conversation on screen is suppressed in the
    // helper via the active tab's focus key.
    queue_notification(
        s,
        if is_direct { "dm" } else { "broadcast" },
        &peer_id,
        nickname.as_deref(),
        &content,
        None,
    );
    let msg_id_for_db = msg_id.clone();
    if is_direct {
        apply_dm_to_state(
            s,
            &body,
            &peer_id,
            latency.as_deref(),
            msg_id,
            reply_to.as_ref(),
        );
    } else {
        apply_broadcast_to_state(
            s,
            &body,
            &peer_id,
            latency.as_deref(),
            msg_id,
            reply_to.as_ref(),
        );
    }
    p2plog_debug(format!(
        "{} from {sender_display}: {content}",
        if is_direct { "DM" } else { "Broadcast" }
    ));
    let meta = p2p_app::MessageMeta {
        sender_nickname: nickname,
        msg_id: msg_id_for_db,
        sent_at: None,
        attachment,
        reply_to,
        encrypted,
    };
    if let Err(e) = p2p_app::save_message_with_meta(
        &content,
        Some(&peer_id),
        &s.topic_str,
        is_direct,
        is_direct.then_some(&peer_id),
        meta,
    ) {
        p2plog_debug(format!("Failed to save: {e}"));
    }
    upsert_peer_last_seen(s, &peer_id, chrono::Utc::now().naive_utc());
    // The Contacts rows carry last-seen too; rebuild and re-sort so an incoming
    // message moves its sender to the top of the Contacts tab immediately, the
    // same way a connection refresh already does.
    s.resort_peers();
}

/// Processes network (swarm) events and updates application state
async fn process_swarm_event(
    swarm_event: SwarmEvent,
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) {
    match swarm_event {
        SwarmEvent::BroadcastMessage(msg) => {
            let mut s = state.lock().await;
            handle_incoming_message(&mut s, msg, false);
            drop(s);
        }
        SwarmEvent::DirectMessage(msg) => {
            let mut s = state.lock().await;
            handle_incoming_message(&mut s, msg, true);
            drop(s);
        }
        SwarmEvent::GroupMessage(msg) => handle_group_message(state, msg).await,
        SwarmEvent::GroupInvite {
            group_id,
            inviter_peer,
        } => handle_group_invite(state, swarm_cmd_tx, group_id, inviter_peer).await,
        SwarmEvent::Receipt {
            peer_id, ack_for, ..
        } => handle_receipt(state, peer_id, ack_for).await,
        SwarmEvent::Typing { peer_id, is_typing } => {
            let mut s = state.lock().await;
            s.receive_typing(&peer_id, is_typing);
            drop(s);
        }
        SwarmEvent::Reaction { msg_id } => handle_reaction_event(state, msg_id).await,
        SwarmEvent::PeerConnected(peer_id) => {
            handle_peer_connected(state, swarm_cmd_tx, peer_id).await;
        }
        SwarmEvent::PeerDisconnected(peer_id) => handle_peer_disconnected(state, peer_id).await,
        SwarmEvent::ListenAddrEstablished(addr) => {
            p2plog_debug(format!("Listening on: {addr}"));
        }
        SwarmEvent::ExternalAddressAdded(addr) => {
            p2plog_debug(format!("External address: {addr}"));
        }
        #[cfg(feature = "nat")]
        SwarmEvent::NatStatusChanged(status) => {
            p2plog_debug(format!("NAT status: {status}"));
        }
        #[cfg(feature = "nat")]
        SwarmEvent::HolePunchResult { peer_id, success } => {
            p2plog_debug(format!(
                "Hole punch with {peer_id}: {}",
                if success { "established" } else { "failed" }
            ));
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered { peer_id, .. } => {
            let mut s = state.lock().await;
            apply_peer_discovered_state(&mut s, &peer_id);
            drop(s);
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerExpired { peer_id } => {
            p2plog_debug(format!("Peer expired: {peer_id}"));
        }
        SwarmEvent::DeviceBindingIntro {
            peer_id,
            device_pubkey,
        } => handle_device_binding_intro(state, peer_id, device_pubkey).await,
        SwarmEvent::DeviceBindingReceived { user_id } => {
            handle_device_binding_received(state, user_id).await;
        }
        SwarmEvent::CallInvite { call_id, peer_id } => {
            handle_call_event(state, swarm_cmd_tx, "invite", &call_id, &peer_id).await;
        }
        SwarmEvent::CallAccepted { call_id, peer_id } => {
            handle_call_event(state, swarm_cmd_tx, "accept", &call_id, &peer_id).await;
        }
        SwarmEvent::CallRejected { call_id, peer_id } => {
            handle_call_event(state, swarm_cmd_tx, "reject", &call_id, &peer_id).await;
        }
        SwarmEvent::CallEnded { call_id, peer_id } => {
            handle_call_event(state, swarm_cmd_tx, "end", &call_id, &peer_id).await;
        }
        SwarmEvent::CallMediaFrame { call_id, pcm, .. } => {
            // The frame counter is the transport-level signal in every build;
            // with `tui-audio` the same PCM also feeds the speaker adapter.
            let mut s = state.lock().await;
            if s.call_session
                .as_ref()
                .is_some_and(|session| session.is_active() && session.call_id == call_id)
                && pcm.len() == p2p_app::call_media::SAMPLES_PER_FRAME
            {
                s.call_media.frames_received = s.call_media.frames_received.saturating_add(1);
                s.call_audio.play(&pcm);
            }
        }
        SwarmEvent::FileReceived { .. } => log_file_received_event(swarm_event),
        SwarmEvent::ShareOffered { .. }
        | SwarmEvent::ShareUpdated { .. }
        | SwarmEvent::ShareRejected { .. } => log_share_event(swarm_event),
    }
}

fn log_file_received_event(ev: SwarmEvent) {
    let SwarmEvent::FileReceived {
        peer_id,
        file_id,
        name,
        path,
    } = ev
    else {
        return;
    };
    p2plog_debug(format!(
        "Received file {name} ({file_id}) from {peer_id} at {path}"
    ));
}

/// Share-lane UI lands in a later slice; the core already applied any
/// downloaded manifest before these events were emitted, so logging is enough
/// for the TUI until a Shares tab exists.
fn log_share_event(ev: SwarmEvent) {
    match ev {
        SwarmEvent::ShareOffered { peer_id, offer } => {
            p2plog_debug(format!(
                "Share {share_id} ({name}, {kind:?}, write={public_write}, rev {revision}) offered by {peer_id}",
                share_id = offer.share_id,
                name = offer.name,
                kind = offer.kind,
                public_write = offer.public_write,
                revision = offer.revision
            ));
        }
        SwarmEvent::ShareUpdated {
            peer_id,
            share_id,
            revision,
            written,
            removed,
        } => {
            p2plog_debug(format!(
                "Share {share_id} updated to revision {revision} from {peer_id} ({written} written, {removed} removed)"
            ));
        }
        SwarmEvent::ShareRejected {
            peer_id,
            share_id,
            reason,
        } => {
            p2plog_debug(format!(
                "Share {share_id} submission rejected by {peer_id}: {reason}"
            ));
        }
        other => p2plog_debug(format!("unexpected share event: {other:?}")),
    }
}

/// Refresh the aggregated reactions for a message after a live event. Fetching
/// through the canonical store keeps this identical to a history load and needs
/// no summary shape duplicated in the event envelope.
async fn handle_reaction_event(state: &SharedState, msg_id: String) {
    let Ok(summaries) = p2p_app::reactions::summaries_for(&msg_id) else {
        return;
    };
    let mut s = state.lock().await;
    s.reactions.insert(msg_id, summaries);
}

/// Route a remote call-signaling event into the single `call_session` slot.
/// An `invite` is adopted only while idle — a settled (ended) session never
/// blocks a fresh ring. A live one actively declines any *other* call (one
/// call at a time) by sending `reject` so the other caller stops ringing,
/// matching Flutter's `_handleCallInvite`. A retransmit of the active call's
/// own id is a no-op: rejecting it would abandon our own live call and desync
/// both sides. `accept`/`reject`/`end` transition the matching session, and
/// the slot is dropped once it settles so the caller can ring again.
async fn handle_call_event(
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    kind: &str,
    call_id: &str,
    peer_id: &str,
) {
    // Decide under the lock, send after dropping it — never await with the
    // state guard held (significant-drop-tightening + no lock across a send).
    let mut decline_foreign = false;
    let mut accepted_now = false;
    let mut s = state.lock().await;
    let mut settled = false;
    match kind {
        "invite" => {
            let active = s
                .call_session
                .as_ref()
                .filter(|session| session.is_active())
                .map(|session| session.call_id.clone());
            if let Some(active_id) = active {
                decline_foreign = active_id != call_id;
            } else {
                s.call_session = Some(p2p_app::CallSession::incoming(call_id, peer_id));
                s.call_media = CallMediaStatus::default();
            }
        }
        _ => {
            if let Some(session) = s.call_session.as_mut() {
                let was_in_call = session.state == p2p_app::CallState::InCall;
                let matched =
                    session.on_signal(&p2p_app::CallSignal::new(kind, call_id.to_string()));
                accepted_now = matched
                    && kind == "accept"
                    && !was_in_call
                    && session.state == p2p_app::CallState::InCall;
                settled = !session.is_active();
            }
        }
    }
    if settled {
        s.call_session = None;
        s.call_media = CallMediaStatus::default();
        s.call_audio.stop();
    }
    drop(s);

    // The callee only opens its media session after accepting locally; the
    // caller opens it here on the peer's `accept`, mirroring the Flutter path.
    if accepted_now {
        let _ = swarm_cmd_tx
            .send(SwarmCommand::StartCallMedia {
                call_id: call_id.to_owned(),
                peer_id: peer_id.to_owned(),
            })
            .await;
        let s = state.lock().await;
        s.call_audio.start(call_id, peer_id, swarm_cmd_tx);
        drop(s);
    }

    if decline_foreign {
        let _ = swarm_cmd_tx
            .send(SwarmCommand::SendCallSignal {
                peer_id: peer_id.to_string(),
                signal: Box::new(p2p_app::CallSignal::new("reject", call_id.to_string())),
            })
            .await;
    }

    p2plog_debug(match kind {
        "invite" => format!("Incoming call {call_id} from {peer_id}"),
        "accept" => format!("Call {call_id} accepted by {peer_id}"),
        "reject" => format!("Call {call_id} declined by {peer_id}"),
        _ => format!("Call {call_id} ended with {peer_id}"),
    });
}

/// Incoming group message: display-name cache + per-group chat state render.
async fn handle_group_message(state: &SharedState, msg: p2p_app::GroupMessageEvent) {
    let mut s = state.lock().await;
    if let Some(n) = msg.nickname.as_ref() {
        s.received_nicknames.insert(msg.peer_id.clone(), n.clone());
        let _ = p2p_app::set_peer_received_nickname(&msg.peer_id, n);
    }
    let sender_display =
        p2p_app::peer_display_name(&msg.peer_id, &s.local_nicknames, &s.received_nicknames);
    let body = render_message_body(&msg.content, msg.attachment.as_ref());
    apply_group_message_to_state(
        &mut s,
        &msg.group_id,
        &body,
        &msg.peer_id,
        msg.msg_id,
        msg.reply_to.as_ref(),
    );
    queue_notification(
        &mut s,
        "group_message",
        &msg.peer_id,
        msg.nickname.as_deref(),
        &msg.content,
        Some(&msg.group_id),
    );
    s.reload_group_summaries();
    drop(s);
    p2plog_debug(format!(
        "Group message from {sender_display}: {}",
        msg.content
    ));
}

/// A group invite materializes our local membership, then subscribes the swarm
/// to the group so its traffic arrives (mirrors mobile's `accept_group_invite`;
/// without the subscribe, incoming group messages never arrive).
async fn handle_group_invite(
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    group_id: String,
    inviter_peer: String,
) {
    let sender_display = {
        let s = state.lock().await;
        let sender_display =
            p2p_app::peer_display_name(&inviter_peer, &s.local_nicknames, &s.received_nicknames);
        let local_peer_id = s.local_peer_id.clone();
        drop(s);
        // An invite only carries a `group_id`, so materialize the group row
        // first (display_name = group_id when the canonical name isn't known;
        // idempotent for a group we already joined by name) — `group_members`
        // references `groups`, and the group must show up in the Groups tab.
        let _ = p2p_app::groups::ensure_group_for_invite(&group_id);
        let _ = p2p_app::groups::record_group_member(&group_id, &local_peer_id);
        sender_display
    };
    p2plog_debug(format!(
        "Accepted join {group_id} invited by {sender_display}",
    ));
    let _ = swarm_cmd_tx
        .send(SwarmCommand::SubscribeGroup { group_id })
        .await;
}

/// Receipt ack: update the in-memory reply state, then persist it to the DB.
async fn handle_receipt(state: &SharedState, peer_id: String, ack_for: String) {
    let mut s = state.lock().await;
    let at = p2p_app::current_timestamp();

    let is_dm = is_dm_receipt(&s, &ack_for);
    apply_receipt_to_state(&mut s, &ack_for, &peer_id, at, is_dm);
    drop(s);
    let kind = i32::from(is_dm);
    let _ = p2p_app::save_receipt(&ack_for, &peer_id, kind, at);
}

/// Freshly-connected peer: update state, announce our nickname over a DM, and
/// feed the peer-session history used for `get_network_size()`.
async fn handle_peer_connected(
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    peer_id: String,
) {
    let (own_nickname, count) = {
        let mut s = state.lock().await;
        let count = apply_peer_connected_count(&mut s);
        p2plog_debug(format!("Peer connected: {peer_id} (total: {count})"));
        apply_peer_connected_state(&mut s, &peer_id);
        let own_nick = s.own_nickname.clone();
        drop(s);
        (own_nick, count)
    };

    // Feed the peer-session history used for `get_network_size()`.
    let _ = p2p_app::peers::record_peer_session(i32::try_from(count).unwrap_or(i32::MAX));
    let msg_id = p2p_app::gen_msg_id();
    let _ = swarm_cmd_tx
        .send(SwarmCommand::SendDm {
            peer_id,
            content: String::new(),
            nickname: Some(own_nickname),
            msg_id: Some(msg_id),
            ack_for: None,
            attachment: None,
            reply_to: None,
        })
        .await;
}

/// Peer disconnect: count-down + connected-overlay bookkeeping, and feed the
/// peer-session history used for `get_network_size()`.
async fn handle_peer_disconnected(state: &SharedState, peer_id: String) {
    let count = {
        let mut s = state.lock().await;
        let count = apply_peer_disconnected_count(&mut s);
        s.connected
            .on_peer_disconnected(&peer_id, p2p_app::current_timestamp());
        s.clear_typing(&peer_id);
        count
    };
    p2plog_debug(format!("Peer disconnected: {peer_id} (total: {count})"));

    // Feed the peer-session history used for `get_network_size()`.
    let _ = p2p_app::peers::record_peer_session(i32::try_from(count).unwrap_or(i32::MAX));
}

/// Device-binding intro: surface it as an explicit prompt for the account
/// holder (never auto-mint) — accepting mints a signed claim via the 'a' key
/// path in the input processor; a single-slot prompt drops a second one.
async fn handle_device_binding_intro(state: &SharedState, peer_id: String, device_pubkey: Vec<u8>) {
    let mut s = state.lock().await;
    if s.device_binding_prompt.is_none() {
        s.device_binding_prompt = Some(super::state::DeviceBindingPrompt {
            peer_id,
            device_pubkey,
        });
    } else {
        p2plog_debug("Device-binding intro dropped: a prompt is already active".to_string());
    }
    drop(s);
}

/// Our install was bound into a foreign user: confirm via popup and re-resolve
/// the identity shown on the Settings tab.
async fn handle_device_binding_received(state: &SharedState, user_id: String) {
    p2plog_debug(format!("This device was bound into user {user_id}"));
    let mut s = state.lock().await;
    s.popup = Some(format!(
        "This device was bound into user {}",
        p2p_app::fmt::peer_id_suffix(&user_id)
    ));
    s.refresh_my_user();
}

pub fn spawn_command_processor(
    state: SharedState,
    mut input_rx: mpsc::Receiver<InputEvent>,
    mut swarm_event_rx: mpsc::Receiver<SwarmEvent>,
    render_tx: mpsc::Sender<RenderEvent>,
    swarm_cmd_tx: mpsc::Sender<SwarmCommand>,
) -> (tokio::task::JoinHandle<()>, mpsc::Sender<SwarmCommand>) {
    let cmd_tx_for_return = swarm_cmd_tx.clone();

    // Typing TTLs are wall-clock state, not input events: a small detached task
    // expires them and asks for a redraw when an indicator disappears.
    let typing_state = Arc::clone(&state);
    let typing_render_tx = render_tx.clone();
    drop(tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(500));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            if typing_state.lock().await.expire_typing()
                && typing_render_tx.send(RenderEvent).await.is_err()
            {
                break;
            }
        }
    }));

    let handle = tokio::spawn(async move {
        p2plog_debug("CommandProcessor task started".to_string());

        loop {
            let event = tokio::select! {
                Some(input_event) = input_rx.recv() => Some(Event::Input(input_event)),
                Some(swarm_event) = swarm_event_rx.recv() => {
                    Some(Event::SwarmEvent(Box::new(swarm_event)))
                },
                else => None,
            };

            match event {
                Some(Event::Input(input_event)) => {
                    if process_input_event(input_event, &state, &swarm_cmd_tx, &render_tx).await {
                        break;
                    }
                }
                Some(Event::SwarmEvent(swarm_event)) => {
                    process_swarm_event(*swarm_event, &state, &swarm_cmd_tx).await;
                    let _ = render_tx.send(RenderEvent).await;
                }
                None => break,
            }
        }
    });

    (handle, cmd_tx_for_return)
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_command_processor.rs"]
mod tests;
