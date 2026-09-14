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
    msg_id: String,
) {
    let msg = format!("{ts} [{own_nickname}] {content}");
    state.messages.push_back(DisplayMessage {
        text: msg,
        sender_peer_id: None,
    });
    state.message_ids.push_back(Some(msg_id));
    trim_history(&mut state.messages, MAX_MESSAGE_HISTORY);
    trim_history(&mut state.message_ids, MAX_MESSAGE_HISTORY);
}

/// Pure: format and push an outgoing DM to state, trimming history
fn push_outgoing_dm_to_state(
    state: &mut AppState,
    peer_id: &str,
    ts: &str,
    dm_self_nickname: &str,
    content: &str,
    msg_id: String,
) {
    let msg = format!("{ts} [{dm_self_nickname}] {content}");
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
    msg_id: String,
) {
    let msg = format!("{ts} [{own_nickname}] {content}");
    let msgs = state.group_messages.entry(group_id.to_string()).or_default();
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

/// Sends a message (broadcast, direct, or group message)
#[allow(clippy::too_many_lines)]
pub async fn send_message(
    state: &mut AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    text: String,
    tab_content: p2p_app::tui_tabs::TabContent,
) {
    let (topic_str, own_nickname) = (state.topic_str.clone(), state.own_nickname.clone());
    let is_direct = matches!(tab_content, p2p_app::tui_tabs::TabContent::Direct(_));
    let is_group = matches!(tab_content, p2p_app::tui_tabs::TabContent::GroupChat(_));
    let dm_target_peer_id: Option<String> =
        if let p2p_app::tui_tabs::TabContent::Direct(pid) = &tab_content {
            Some(pid.clone())
        } else {
            None
        };
    let group_target_id: Option<String> =
        if let p2p_app::tui_tabs::TabContent::GroupChat(gid) = &tab_content {
            Some(gid.clone())
        } else {
            None
        };
    let ts = p2p_app::format_now();
    let dm_self_nickname = dm_target_peer_id
        .as_deref()
        .and_then(|pid| state.self_nicknames_for_peers.get(pid).cloned())
        .unwrap_or_else(|| own_nickname.clone());
    let msg_id = p2p_app::gen_msg_id();
    let msg_id_for_db = msg_id.clone();
    let sent_at = p2p_app::current_timestamp();

    if is_direct {
        if let Some(ref peer_id) = dm_target_peer_id {
            push_outgoing_dm_to_state(
                state,
                peer_id,
                &ts,
                &dm_self_nickname,
                &text,
                msg_id.clone(),
            );
            p2plog_debug(format!("Sent DM to {peer_id}: {text}"));
        }
    } else if is_group {
        if let Some(ref group_id) = group_target_id {
            push_outgoing_group_message_to_state(
                state,
                group_id,
                &ts,
                &own_nickname,
                &text,
                msg_id.clone(),
            );
            p2plog_debug(format!("Sent group message to {group_id}: {text}"));
        }
    } else {
        push_outgoing_broadcast_to_state(state, &ts, &own_nickname, &text, msg_id.clone());
        p2plog_debug(format!("Sent broadcast: {text}"));
    }

    state.chat_input = ratatui_textarea::TextArea::default();

    if is_direct {
        if let Some(peer_id) = dm_target_peer_id.clone() {
            let _ = swarm_cmd_tx
                .send(SwarmCommand::SendDm {
                    peer_id,
                    content: text.clone(),
                    nickname: Some(dm_self_nickname),
                    msg_id: Some(msg_id),
                    ack_for: None,
                })
                .await;
        }
        let meta = p2p_app::MessageMeta {
            sender_nickname: Some(own_nickname.clone()),
            msg_id: Some(msg_id_for_db.clone()),
            sent_at: Some(sent_at),
        };
        if let Err(e) = p2p_app::save_message_with_meta(
            &text,
            None,
            &topic_str,
            true,
            dm_target_peer_id.as_deref(),
            meta,
        ) {
            p2plog_debug(format!("Failed to save message: {e}"));
        }
    } else if is_group {
        if let Some(group_id) = group_target_id.clone() {
            let _ = swarm_cmd_tx
                .send(SwarmCommand::PublishGroup {
                    group_id,
                    content: text.clone(),
                    nickname: Some(own_nickname.clone()),
                    msg_id: Some(msg_id),
                })
                .await;
        }
        if let Some(group_id) = group_target_id {
            let meta = p2p_app::groups::GroupMessageMeta {
                sender_nickname: Some(own_nickname),
                msg_id: Some(msg_id_for_db.clone()),
                sent_at: Some(sent_at),
            };
            if let Err(e) =
                p2p_app::groups::save_outgoing_group_message(&group_id, &text, meta)
            {
                p2plog_debug(format!("Failed to save group message: {e}"));
            }
        }
    } else {
        let _ = swarm_cmd_tx
            .send(SwarmCommand::Publish {
                content: text.clone(),
                nickname: Some(own_nickname.clone()),
                msg_id: Some(msg_id),
            })
            .await;
        let meta = p2p_app::MessageMeta {
            sender_nickname: Some(own_nickname),
            msg_id: Some(msg_id_for_db.clone()),
            sent_at: Some(sent_at),
        };
        if let Err(e) = p2p_app::save_message_with_meta(&text, None, &topic_str, false, None, meta)
        {
            p2plog_debug(format!("Failed to save message: {e}"));
        }

        // Attribute outgoing broadcasts to every peer that was online to receive
        // them, so the peers table can show how many broadcasts we sent each peer.
        let recipients: Vec<String> = state
            .connected
            .connected_peer_ids()
            .map(str::to_owned)
            .collect();
        if !recipients.is_empty() {
            let _ = p2p_app::peers::record_broadcast_recipients(&msg_id_for_db, &recipients);
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_message_handlers.rs"]
mod tests;
