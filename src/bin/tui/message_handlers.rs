use super::constants::{MAX_DM_HISTORY, MAX_MESSAGE_HISTORY};
use super::state::AppState;
use p2p_app::{SwarmCommand, p2plog_debug};
use std::time::SystemTime;
use tokio::sync::mpsc;

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
    let ts = p2p_app::format_system_time(SystemTime::now());
    let dm_self_nickname = dm_target_peer_id
        .as_deref()
        .and_then(|pid| state.self_nicknames_for_peers.get(pid).cloned())
        .unwrap_or_else(|| own_nickname.clone());
    let msg_id = p2p_app::gen_msg_id();
    let msg_id_for_db = msg_id.clone();
    let sent_at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    state.sent_at_by_msg_id.insert(msg_id.clone(), sent_at);

    if is_direct {
        if let Some(ref peer_id) = dm_target_peer_id {
            let msg = format!("{} [{}] {}", ts, dm_self_nickname, &text);
            let dm_msgs = state.dm_messages.entry(peer_id.clone()).or_default();
            dm_msgs.push_back(msg);
            state
                .dm_message_ids
                .entry(peer_id.clone())
                .or_default()
                .push_back(Some(msg_id.clone()));
            if dm_msgs.len() > MAX_DM_HISTORY {
                dm_msgs.pop_front();
                if let Some(ids) = state.dm_message_ids.get_mut(peer_id) {
                    let _ = ids.pop_front();
                }
            }
            p2plog_debug(format!("Sent DM to {}: {}", peer_id, text));
        }
    } else {
        let msg = format!("{} [{}] {}", ts, own_nickname, &text);
        state.messages.push_back((msg, None));
        state.message_ids.push_back(Some(msg_id.clone()));
        if state.messages.len() > MAX_MESSAGE_HISTORY {
            state.messages.pop_front();
            let _ = state.message_ids.pop_front();
        }
        p2plog_debug(format!("Sent broadcast: {}", text));
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
    // For direct messages, `peer_id` in DB represents the sender. Outgoing DM sender is us -> store None.
    let db_sender_peer_id = if is_direct { None } else { peer_ref };
    // Use own nickname as sender's nickname for outgoing messages
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
        p2plog_debug(format!("Failed to save message: {}", e));
    }
}
