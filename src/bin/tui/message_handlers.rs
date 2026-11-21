use super::state::AppState;
use super::state::{MAX_DM_HISTORY, MAX_MESSAGE_HISTORY, trim_history};
use p2p_app::{DisplayMessage, SwarmCommand, p2plog_debug};
use std::collections::VecDeque;
use tokio::sync::mpsc;

/// Attach the DB row id of a just-sent message to its aligned queue slot.
/// The push happens before the DB write (to preserve the visible order), so
/// the caller records the row index and fills it in once the save succeeds.
fn attach_sent_db_id(ids: Option<&mut VecDeque<Option<i32>>>, idx: usize, id: i32) {
    if let Some(ids) = ids
        && let Some(slot) = ids.get_mut(idx)
    {
        *slot = Some(id);
    }
}

/// Pure: format and push an outgoing broadcast message to state, trimming history.
/// Returns the index of the new row so the caller can attach its saved DB id.
fn push_outgoing_broadcast_to_state(
    state: &mut AppState,
    ts: &str,
    own_nickname: &str,
    content: &str,
    attachment: Option<&p2p_app::files::MessageAttachment>,
    msg_id: String,
    reply_to: Option<&p2p_app::ReplyRef>,
) -> usize {
    let body = render_outgoing_body(content, attachment);
    let msg = p2p_app::reply::with_quoted_header(format!("{ts} [{own_nickname}] {body}"), reply_to);
    state.messages.push_back(DisplayMessage {
        text: msg,
        sender_peer_id: None,
    });
    state.message_ids.push_back(Some(msg_id));
    state.message_db_ids.push_back(None);
    trim_history(&mut state.messages, MAX_MESSAGE_HISTORY);
    trim_history(&mut state.message_ids, MAX_MESSAGE_HISTORY);
    trim_history(&mut state.message_db_ids, MAX_MESSAGE_HISTORY);
    state.message_db_ids.len().saturating_sub(1)
}

/// Pure: compose an outgoing chat line before the sender tag: text first, the
/// attachment chip appended; an attachment-only message renders as just the
/// chip so a file never shows up as a blank row.
fn render_outgoing_body(
    content: &str,
    attachment: Option<&p2p_app::files::MessageAttachment>,
) -> String {
    let chip = attachment.map_or_else(String::new, p2p_app::files::file_chip);
    if content.trim().is_empty() {
        chip
    } else if chip.is_empty() {
        content.to_string()
    } else {
        format!("{content} {chip}")
    }
}

/// Inputs for appending an outgoing DM to the in-memory chat log.
struct OutgoingDm<'a> {
    peer_id: &'a str,
    ts: &'a str,
    self_nickname: &'a str,
    content: &'a str,
    attachment: Option<&'a p2p_app::files::MessageAttachment>,
    msg_id: String,
    reply_to: Option<&'a p2p_app::ReplyRef>,
    encrypted: bool,
}

/// Pure: format and push an outgoing DM to state, trimming history. Returns the
/// index of the new row so the caller can attach its saved DB id.
fn push_outgoing_dm_to_state(state: &mut AppState, dm: OutgoingDm<'_>) -> usize {
    let OutgoingDm {
        peer_id,
        ts,
        self_nickname,
        content,
        attachment,
        msg_id,
        reply_to,
        encrypted,
    } = dm;
    let body = p2p_app::e2e::mark_encrypted(render_outgoing_body(content, attachment), encrypted);
    let msg =
        p2p_app::reply::with_quoted_header(format!("{ts} [{self_nickname}] {body}"), reply_to);
    let dm_msgs = state.dm_messages.entry(peer_id.to_string()).or_default();
    dm_msgs.push_back(msg);
    state
        .dm_message_ids
        .entry(peer_id.to_string())
        .or_default()
        .push_back(Some(msg_id));
    state
        .dm_message_db_ids
        .entry(peer_id.to_string())
        .or_default()
        .push_back(None);
    trim_history(dm_msgs, MAX_DM_HISTORY);
    if let Some(ids) = state.dm_message_ids.get_mut(peer_id) {
        trim_history(ids, MAX_DM_HISTORY);
    }
    if let Some(ids) = state.dm_message_db_ids.get_mut(peer_id) {
        trim_history(ids, MAX_DM_HISTORY);
    }
    state
        .dm_message_db_ids
        .get(peer_id)
        .map_or(0, VecDeque::len)
        .saturating_sub(1)
}

/// Inputs for appending an outgoing group message to the in-memory chat log.
struct OutgoingGroup<'a> {
    group_id: &'a str,
    ts: &'a str,
    own_nickname: &'a str,
    content: &'a str,
    attachment: Option<&'a p2p_app::files::MessageAttachment>,
    msg_id: String,
    reply_to: Option<&'a p2p_app::ReplyRef>,
}

/// Pure: format and push an outgoing group message to state, trimming history.
/// Returns the index of the new row so the caller can attach its saved DB id.
fn push_outgoing_group_message_to_state(state: &mut AppState, group: OutgoingGroup<'_>) -> usize {
    let OutgoingGroup {
        group_id,
        ts,
        own_nickname,
        content,
        attachment,
        msg_id,
        reply_to,
    } = group;
    let body = render_outgoing_body(content, attachment);
    let msg = p2p_app::reply::with_quoted_header(format!("{ts} [{own_nickname}] {body}"), reply_to);
    let msgs = state
        .group_messages
        .entry(group_id.to_string())
        .or_default();
    msgs.push_back(msg);
    state
        .group_message_ids
        .entry(group_id.to_string())
        .or_default()
        .push_back(Some(msg_id));
    state
        .group_message_db_ids
        .entry(group_id.to_string())
        .or_default()
        .push_back(None);
    state
        .group_message_peer_ids
        .entry(group_id.to_string())
        .or_default()
        .push_back(None);
    trim_history(msgs, MAX_MESSAGE_HISTORY);
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
        let len = msgs.len();
        (len, true)
    });
    state
        .group_message_db_ids
        .get(group_id)
        .map_or(0, VecDeque::len)
        .saturating_sub(1)
}

/// Sends a message (broadcast, direct, or group message): pushes it into the
/// active chat state, dispatches it over the swarm, and persists it with
/// receipt metadata. Routing picks the per-kind helper by the active tab.
///
/// `attachment` rides along as a descriptor (files lane §12); the bytes were
/// already stored via `files::store_outgoing_file` so the receiving end can
/// pull them from us.
pub async fn send_message(
    state: &mut AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    text: String,
    attachment: Option<p2p_app::files::MessageAttachment>,
    tab_content: p2p_app::tui_tabs::TabContent,
) {
    let own_nickname = state.own_nickname.clone();
    // One reply per outgoing message: the draft is consumed here so a failed
    // send still clears the affordance instead of silently quoting twice.
    let reply_to = state.take_reply(&tab_content);
    match &tab_content {
        p2p_app::tui_tabs::TabContent::Direct(peer_id) => {
            send_dm_message(
                state,
                swarm_cmd_tx,
                peer_id.clone(),
                text,
                attachment,
                own_nickname,
                reply_to,
            )
            .await;
        }
        p2p_app::tui_tabs::TabContent::GroupChat(group_id) => {
            send_group_message(
                state,
                swarm_cmd_tx,
                group_id.clone(),
                text,
                attachment,
                own_nickname,
                reply_to,
            )
            .await;
        }
        _ => {
            send_broadcast_message(
                state,
                swarm_cmd_tx,
                text,
                attachment,
                own_nickname,
                reply_to,
            )
            .await;
        }
    }
}

/// Attach a local file to a message on the active tab and send it (files lane
/// §12). The bytes are content-addressed into our store first so senders can
/// serve the receiver's pull; failures surface in a popup and the input is
/// kept as-is.
pub async fn send_file_message(
    state: &mut AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    path: &str,
    tab_content: p2p_app::tui_tabs::TabContent,
) {
    match p2p_app::files::store_outgoing_file(path) {
        Ok(attachment) => {
            send_message(
                state,
                swarm_cmd_tx,
                String::new(),
                Some(attachment),
                tab_content,
            )
            .await;
        }
        Err(e) => {
            state.popup = Some(format!("Failed to attach {path}: {e}"));
        }
    }
}

/// Send a transient typing state for a DM conversation (US-009).
///
/// Outgoing "started" signals are throttled in [`AppState`]; "stopped" signals
/// are always sent so a remote indicator is cleared immediately.
pub fn send_typing(
    state: &mut AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    peer_id: &str,
    is_typing: bool,
) {
    if !state.note_typing_sent(peer_id, is_typing) {
        return;
    }
    let _ = swarm_cmd_tx.try_send(SwarmCommand::SendTyping {
        peer_id: peer_id.to_string(),
        is_typing,
    });
}

/// Send a direct message: push to the DM chat state, dispatch over the swarm,
/// and persist with receipt metadata. The sender nickname is the per-peer
/// override when one exists, else our own nickname.
async fn send_dm_message(
    state: &mut AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    peer_id: String,
    text: String,
    attachment: Option<p2p_app::files::MessageAttachment>,
    own_nickname: String,
    reply_to: Option<p2p_app::ReplyRef>,
) {
    let ts = p2p_app::format_now();
    let dm_self_nickname = state
        .self_nicknames_for_peers
        .get(&peer_id)
        .cloned()
        .unwrap_or_else(|| own_nickname.clone());
    let msg_id = p2p_app::gen_msg_id();
    let msg_id_for_db = msg_id.clone();
    let sent_at = p2p_app::current_timestamp();

    let encrypted = p2p_app::e2e::can_encrypt_to_str(&peer_id);

    let row_idx = push_outgoing_dm_to_state(
        state,
        OutgoingDm {
            peer_id: &peer_id,
            ts: &ts,
            self_nickname: &dm_self_nickname,
            content: &text,
            attachment: attachment.as_ref(),
            msg_id: msg_id.clone(),
            reply_to: reply_to.as_ref(),
            encrypted,
        },
    );
    p2plog_debug(format!("Sent DM to {peer_id}: {text}"));

    state.chat_input = ratatui_textarea::TextArea::default();

    let _ = swarm_cmd_tx
        .send(SwarmCommand::SendDm {
            peer_id: peer_id.clone(),
            content: text.clone(),
            nickname: Some(dm_self_nickname),
            msg_id: Some(msg_id),
            ack_for: None,
            attachment: attachment.clone(),
            reply_to: reply_to.clone(),
        })
        .await;
    // Clear the remote indicator only after the message itself is queued, so
    // a peer sees "typing…" until the DM arrives. Fire-and-forget: a full lane
    // never blocks the message send.
    send_typing(state, swarm_cmd_tx, &peer_id, false);

    let meta = p2p_app::MessageMeta {
        sender_nickname: Some(own_nickname),
        msg_id: Some(msg_id_for_db),
        sent_at: Some(sent_at),
        attachment,
        reply_to,
        encrypted,
    };
    let saved_id = match p2p_app::save_message_with_meta(
        &text,
        None,
        &state.topic_str,
        true,
        Some(&peer_id),
        meta,
    ) {
        Ok(saved) => Some(saved.id),
        Err(e) => {
            p2plog_debug(format!("Failed to save message: {e}"));
            None
        }
    };
    // The TUI is the sender: stamp `sent = 1` so the row is deletable (US-005).
    let sent_id = saved_id.and_then(|id| match p2p_app::messages::mark_message_sent(id) {
        Ok(()) => Some(id),
        Err(e) => {
            p2plog_debug(format!("Failed to mark message as sent: {e}"));
            None
        }
    });
    if let Some(id) = sent_id {
        attach_sent_db_id(state.dm_message_db_ids.get_mut(&peer_id), row_idx, id);
    }
}

/// Send a group message: push to the group chat state, dispatch over the
/// swarm, and persist with group metadata.
async fn send_group_message(
    state: &mut AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    group_id: String,
    text: String,
    attachment: Option<p2p_app::files::MessageAttachment>,
    own_nickname: String,
    reply_to: Option<p2p_app::ReplyRef>,
) {
    let ts = p2p_app::format_now();
    let msg_id = p2p_app::gen_msg_id();
    let msg_id_for_db = msg_id.clone();
    let sent_at = p2p_app::current_timestamp();

    let row_idx = push_outgoing_group_message_to_state(
        state,
        OutgoingGroup {
            group_id: &group_id,
            ts: &ts,
            own_nickname: &own_nickname,
            content: &text,
            attachment: attachment.as_ref(),
            msg_id: msg_id.clone(),
            reply_to: reply_to.as_ref(),
        },
    );
    p2plog_debug(format!("Sent group message to {group_id}: {text}"));

    state.chat_input = ratatui_textarea::TextArea::default();

    let _ = swarm_cmd_tx
        .send(SwarmCommand::PublishGroup {
            group_id: group_id.clone(),
            content: text.clone(),
            nickname: Some(own_nickname.clone()),
            msg_id: Some(msg_id),
            attachment: attachment.clone(),
            reply_to: reply_to.clone(),
        })
        .await;

    let meta = p2p_app::groups::GroupMessageMeta {
        sender_nickname: Some(own_nickname),
        msg_id: Some(msg_id_for_db),
        sent_at: Some(sent_at),
        attachment,
        reply_to,
    };
    // Group outgoing rows are inserted with `sent = 1` already; just capture
    // the id so the local-only Delete binding can find the row (US-005).
    match p2p_app::groups::save_outgoing_group_message(&group_id, &text, meta) {
        Ok(saved) => attach_sent_db_id(
            state.group_message_db_ids.get_mut(&group_id),
            row_idx,
            saved.id,
        ),
        Err(e) => p2plog_debug(format!("Failed to save group message: {e}")),
    }
}

/// Send a broadcast: push to the broadcast chat state, dispatch over the
/// swarm, persist with metadata, and attribute the message to every peer that
/// was online to receive it so the peers table can show how many broadcasts we
/// sent each peer.
async fn send_broadcast_message(
    state: &mut AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    text: String,
    attachment: Option<p2p_app::files::MessageAttachment>,
    own_nickname: String,
    reply_to: Option<p2p_app::ReplyRef>,
) {
    let ts = p2p_app::format_now();
    let msg_id = p2p_app::gen_msg_id();
    let msg_id_for_db = msg_id.clone();
    let sent_at = p2p_app::current_timestamp();

    let row_idx = push_outgoing_broadcast_to_state(
        state,
        &ts,
        &own_nickname,
        &text,
        attachment.as_ref(),
        msg_id.clone(),
        reply_to.as_ref(),
    );
    p2plog_debug(format!("Sent broadcast: {text}"));

    state.chat_input = ratatui_textarea::TextArea::default();

    let _ = swarm_cmd_tx
        .send(SwarmCommand::Publish {
            content: text.clone(),
            nickname: Some(own_nickname.clone()),
            msg_id: Some(msg_id),
            attachment: attachment.clone(),
            reply_to: reply_to.clone(),
        })
        .await;

    let meta = p2p_app::MessageMeta {
        sender_nickname: Some(own_nickname),
        msg_id: Some(msg_id_for_db.clone()),
        sent_at: Some(sent_at),
        attachment,
        reply_to,
        encrypted: false,
    };
    let saved_id =
        match p2p_app::save_message_with_meta(&text, None, &state.topic_str, false, None, meta) {
            Ok(saved) => Some(saved.id),
            Err(e) => {
                p2plog_debug(format!("Failed to save message: {e}"));
                None
            }
        };
    let sent_id = saved_id.and_then(|id| match p2p_app::messages::mark_message_sent(id) {
        Ok(()) => Some(id),
        Err(e) => {
            p2plog_debug(format!("Failed to mark message as sent: {e}"));
            None
        }
    });
    if let Some(id) = sent_id {
        attach_sent_db_id(Some(&mut state.message_db_ids), row_idx, id);
    }

    let recipients: Vec<String> = state
        .connected
        .connected_peer_ids()
        .map(str::to_owned)
        .collect();
    if !recipients.is_empty() {
        let _ = p2p_app::peers::record_broadcast_recipients(&msg_id_for_db, &recipients);
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_message_handlers.rs"]
mod tests;
