use super::state::AppState;
use super::state::{MAX_DM_HISTORY, MAX_MESSAGE_HISTORY, trim_history};
use p2p_app::{DisplayMessage, SwarmCommand, p2plog_debug};
use tokio::sync::mpsc;

/// Pure: format and push an outgoing broadcast message to state, trimming history
fn push_outgoing_broadcast_to_state(
    state: &mut AppState,
    ts: &str,
    own_nickname: &str,
    content: &str,
    attachment: Option<&p2p_app::files::MessageAttachment>,
    msg_id: String,
) {
    let body = render_outgoing_body(content, attachment);
    let msg = format!("{ts} [{own_nickname}] {body}");
    state.messages.push_back(DisplayMessage {
        text: msg,
        sender_peer_id: None,
    });
    state.message_ids.push_back(Some(msg_id));
    trim_history(&mut state.messages, MAX_MESSAGE_HISTORY);
    trim_history(&mut state.message_ids, MAX_MESSAGE_HISTORY);
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

/// Pure: format and push an outgoing DM to state, trimming history
fn push_outgoing_dm_to_state(
    state: &mut AppState,
    peer_id: &str,
    ts: &str,
    dm_self_nickname: &str,
    content: &str,
    attachment: Option<&p2p_app::files::MessageAttachment>,
    msg_id: String,
    encrypted: bool,
) {
    let body = p2p_app::e2e::mark_encrypted(render_outgoing_body(content, attachment), encrypted);
    let msg = format!("{ts} [{dm_self_nickname}] {body}");
    let dm_msgs = state.dm_messages.entry(peer_id.to_string()).or_default();
    dm_msgs.push_back(msg);
    state
        .dm_message_ids
        .entry(peer_id.to_string())
        .or_default()
        .push_back(Some(msg_id));
    trim_history(dm_msgs, MAX_DM_HISTORY);
    if let Some(ids) = state.dm_message_ids.get_mut(peer_id) {
        trim_history(ids, MAX_DM_HISTORY);
    }
}

/// Pure: format and push an outgoing group message to state, trimming history
fn push_outgoing_group_message_to_state(
    state: &mut AppState,
    group_id: &str,
    ts: &str,
    own_nickname: &str,
    content: &str,
    attachment: Option<&p2p_app::files::MessageAttachment>,
    msg_id: String,
) {
    let body = render_outgoing_body(content, attachment);
    let msg = format!("{ts} [{own_nickname}] {body}");
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
        .group_message_peer_ids
        .entry(group_id.to_string())
        .or_default()
        .push_back(None);
    trim_history(msgs, MAX_MESSAGE_HISTORY);
    if let Some(ids) = state.group_message_ids.get_mut(group_id) {
        trim_history(ids, MAX_MESSAGE_HISTORY);
    }
    if let Some(ids) = state.group_message_peer_ids.get_mut(group_id) {
        trim_history(ids, MAX_MESSAGE_HISTORY);
    }
    state.group_scroll_state.insert(group_id.to_string(), {
        let len = msgs.len();
        (len, true)
    });
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
    match &tab_content {
        p2p_app::tui_tabs::TabContent::Direct(peer_id) => {
            send_dm_message(
                state,
                swarm_cmd_tx,
                peer_id.clone(),
                text,
                attachment,
                own_nickname,
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
            )
            .await;
        }
        _ => send_broadcast_message(state, swarm_cmd_tx, text, attachment, own_nickname).await,
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

    push_outgoing_dm_to_state(
        state,
        &peer_id,
        &ts,
        &dm_self_nickname,
        &text,
        attachment.as_ref(),
        msg_id.clone(),
        encrypted,
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
        })
        .await;

    let meta = p2p_app::MessageMeta {
        sender_nickname: Some(own_nickname),
        msg_id: Some(msg_id_for_db),
        sent_at: Some(sent_at),
        attachment,
        encrypted,
    };
    if let Err(e) =
        p2p_app::save_message_with_meta(&text, None, &state.topic_str, true, Some(&peer_id), meta)
    {
        p2plog_debug(format!("Failed to save message: {e}"));
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
) {
    let ts = p2p_app::format_now();
    let msg_id = p2p_app::gen_msg_id();
    let msg_id_for_db = msg_id.clone();
    let sent_at = p2p_app::current_timestamp();

    push_outgoing_group_message_to_state(
        state,
        &group_id,
        &ts,
        &own_nickname,
        &text,
        attachment.as_ref(),
        msg_id.clone(),
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
        })
        .await;

    let meta = p2p_app::groups::GroupMessageMeta {
        sender_nickname: Some(own_nickname),
        msg_id: Some(msg_id_for_db),
        sent_at: Some(sent_at),
        attachment,
    };
    if let Err(e) = p2p_app::groups::save_outgoing_group_message(&group_id, &text, meta) {
        p2plog_debug(format!("Failed to save group message: {e}"));
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
) {
    let ts = p2p_app::format_now();
    let msg_id = p2p_app::gen_msg_id();
    let msg_id_for_db = msg_id.clone();
    let sent_at = p2p_app::current_timestamp();

    push_outgoing_broadcast_to_state(
        state,
        &ts,
        &own_nickname,
        &text,
        attachment.as_ref(),
        msg_id.clone(),
    );
    p2plog_debug(format!("Sent broadcast: {text}"));

    state.chat_input = ratatui_textarea::TextArea::default();

    let _ = swarm_cmd_tx
        .send(SwarmCommand::Publish {
            content: text.clone(),
            nickname: Some(own_nickname.clone()),
            msg_id: Some(msg_id),
            attachment: attachment.clone(),
        })
        .await;

    let meta = p2p_app::MessageMeta {
        sender_nickname: Some(own_nickname),
        msg_id: Some(msg_id_for_db.clone()),
        sent_at: Some(sent_at),
        attachment,
        encrypted: false,
    };
    if let Err(e) =
        p2p_app::save_message_with_meta(&text, None, &state.topic_str, false, None, meta)
    {
        p2plog_debug(format!("Failed to save message: {e}"));
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
