use super::constants::{MAX_DM_HISTORY, MAX_MESSAGE_HISTORY};
use super::state::AppState;
use p2p_app::{SwarmCommand, p2plog_debug};
use std::time::SystemTime;
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
    state.messages.push_back((msg, None));
    state.message_ids.push_back(Some(msg_id));
    if state.messages.len() > MAX_MESSAGE_HISTORY {
        state.messages.pop_front();
        let _ = state.message_ids.pop_front();
    }
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
    if dm_msgs.len() > MAX_DM_HISTORY {
        dm_msgs.pop_front();
        if let Some(ids) = state.dm_message_ids.get_mut(peer_id) {
            let _ = ids.pop_front();
        }
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
mod tests {
    use super::*;
    use crate::tui::test_helpers::test_app_state;

    // ── push_outgoing_broadcast_to_state ─────────────────────────────────────

    #[test]
    fn test_push_outgoing_broadcast_adds_message() {
        let mut state = test_app_state();
        push_outgoing_broadcast_to_state(&mut state, "12:00", "Me", "hello", "m1".into(), 1.0);
        assert_eq!(state.messages.len(), 1);
        assert!(state.messages[0].0.contains("[Me]"));
        assert!(state.messages[0].0.contains("hello"));
        assert_eq!(state.messages[0].1, None);
        assert_eq!(state.message_ids[0], Some("m1".to_string()));
        assert_eq!(state.sent_at_by_msg_id.get("m1"), Some(&1.0));
    }

    #[test]
    fn test_push_outgoing_broadcast_trims_history() {
        let mut state = test_app_state();
        for i in 0..MAX_MESSAGE_HISTORY {
            push_outgoing_broadcast_to_state(
                &mut state,
                "12:00",
                "Me",
                &format!("msg-{i}"),
                format!("m{i}"),
                i as f64,
            );
        }
        assert_eq!(state.messages.len(), MAX_MESSAGE_HISTORY);
        push_outgoing_broadcast_to_state(
            &mut state,
            "12:00",
            "Me",
            "newest",
            "m-last".into(),
            99.0,
        );
        assert_eq!(state.messages.len(), MAX_MESSAGE_HISTORY);
        assert!(state.messages[MAX_MESSAGE_HISTORY - 1].0.contains("newest"));
    }

    #[test]
    fn test_push_outgoing_broadcast_peer_id_is_none() {
        let mut state = test_app_state();
        push_outgoing_broadcast_to_state(&mut state, "12:00", "Me", "test", "m1".into(), 1.0);
        assert_eq!(state.messages[0].1, None);
    }

    // ── push_outgoing_dm_to_state ────────────────────────────────────────────

    #[test]
    fn test_push_outgoing_dm_adds_message() {
        let mut state = test_app_state();
        push_outgoing_dm_to_state(
            &mut state,
            "peer-1",
            "12:00",
            "Me",
            "secret",
            "dm1".into(),
            1.0,
        );
        assert!(state.dm_messages.contains_key("peer-1"));
        assert_eq!(state.dm_messages["peer-1"].len(), 1);
        assert!(state.dm_messages["peer-1"][0].contains("[Me]"));
        assert!(state.dm_messages["peer-1"][0].contains("secret"));
        assert_eq!(state.dm_message_ids["peer-1"][0], Some("dm1".to_string()));
        assert_eq!(state.sent_at_by_msg_id.get("dm1"), Some(&1.0));
    }

    #[test]
    fn test_push_outgoing_dm_trims_history() {
        let mut state = test_app_state();
        for i in 0..MAX_DM_HISTORY {
            push_outgoing_dm_to_state(
                &mut state,
                "p1",
                "12:00",
                "Me",
                &format!("msg-{i}"),
                format!("m{i}"),
                i as f64,
            );
        }
        assert_eq!(state.dm_messages["p1"].len(), MAX_DM_HISTORY);
        push_outgoing_dm_to_state(
            &mut state,
            "p1",
            "12:00",
            "Me",
            "newest",
            "m-last".into(),
            99.0,
        );
        assert_eq!(state.dm_messages["p1"].len(), MAX_DM_HISTORY);
        assert!(state.dm_messages["p1"][MAX_DM_HISTORY - 1].contains("newest"));
        assert!(state.dm_messages["p1"][0].contains("msg-1")); // msg-0 was popped
    }

    #[test]
    fn test_push_outgoing_dm_separate_peers() {
        let mut state = test_app_state();
        push_outgoing_dm_to_state(&mut state, "pa", "12:00", "Me", "hi", "m1".into(), 1.0);
        push_outgoing_dm_to_state(&mut state, "pb", "12:00", "Me", "ho", "m2".into(), 2.0);
        assert_eq!(state.dm_messages.len(), 2);
        assert_eq!(state.dm_messages["pa"].len(), 1);
        assert_eq!(state.dm_messages["pb"].len(), 1);
    }
}
