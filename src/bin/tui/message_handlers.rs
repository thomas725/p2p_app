use super::state::AppState;
use super::state::{MAX_DM_HISTORY, MAX_MESSAGE_HISTORY, trim_history};
use p2p_app::{DisplayMessage, SwarmCommand, p2plog_debug};
use tokio::sync::mpsc;

/// Pure: format and push an outgoing broadcast message to state, trimming history
pub fn push_outgoing_broadcast_to_state(
    state: &mut AppState,
    ts: &str,
    own_nickname: &str,
    content: &str,
    msg_id: String,
    sent_at: f64,
) {
    let msg = format!("{} [{}] {}", ts, own_nickname, content);
    state.sent_at_by_msg_id.insert(msg_id.clone(), sent_at);
    state.messages.push_back(DisplayMessage {
        text: msg,
        sender_peer_id: None,
    });
    state.message_ids.push_back(Some(msg_id));
    trim_history(&mut state.messages, MAX_MESSAGE_HISTORY);
    trim_history(&mut state.message_ids, MAX_MESSAGE_HISTORY);
}

/// Pure: format and push an outgoing DM to state, trimming history
pub fn push_outgoing_dm_to_state(
    state: &mut AppState,
    peer_id: &str,
    ts: &str,
    dm_self_nickname: &str,
    content: &str,
    msg_id: String,
    sent_at: f64,
) {
    let msg = format!("{} [{}] {}", ts, dm_self_nickname, content);
    state.sent_at_by_msg_id.insert(msg_id.clone(), sent_at);
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

/// Sends a message (either broadcast or direct message)
pub async fn send_message(
    state: &mut AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    text: String,
    tab_content: p2p_app::tui_tabs::TabContent,
) {
    let (topic_str, own_nickname) = (state.topic_str.clone(), state.own_nickname.clone());
    let is_direct = matches!(tab_content, p2p_app::tui_tabs::TabContent::Direct(_));
    let dm_target_peer_id: Option<String> =
        if let p2p_app::tui_tabs::TabContent::Direct(pid) = &tab_content {
            Some(pid.clone())
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
                sent_at,
            );
            p2plog_debug(format!("Sent DM to {peer_id}: {text}"));
        }
    } else {
        push_outgoing_broadcast_to_state(state, &ts, &own_nickname, &text, msg_id.clone(), sent_at);
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
    } else {
        let _ = swarm_cmd_tx
            .send(SwarmCommand::Publish {
                content: text.clone(),
                nickname: Some(own_nickname.clone()),
                msg_id: Some(msg_id),
            })
            .await;
    }

    let peer_ref = dm_target_peer_id.as_deref();
    let db_sender_peer_id = if is_direct { None } else { peer_ref };
    let meta = p2p_app::MessageMeta {
        sender_nickname: Some(own_nickname),
        msg_id: Some(msg_id_for_db),
        sent_at: Some(sent_at),
    };
    if let Err(e) = p2p_app::save_message_with_meta(
        &text,
        db_sender_peer_id,
        &topic_str,
        is_direct,
        dm_target_peer_id.as_deref(),
        meta,
    ) {
        p2plog_debug(format!("Failed to save message: {e}"));
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_message_handlers.rs"]
mod tests;
