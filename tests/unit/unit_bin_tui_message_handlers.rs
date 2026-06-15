use super::*;
use crate::tui::test_helpers::test_app_state;
use tempfile::TempDir;

// ── push_outgoing_broadcast_to_state ─────────────────────────────────────

#[test]
fn test_push_outgoing_broadcast_adds_message() {
    let mut state = test_app_state();
    push_outgoing_broadcast_to_state(&mut state, "12:00", "Me", "hello", "m1".into(), 1.0);
    assert_eq!(state.messages.len(), 1);
    assert!(state.messages[0].text.contains("[Me]"));
    assert!(state.messages[0].text.contains("hello"));
    assert_eq!(state.messages[0].sender_peer_id, None);
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
    push_outgoing_broadcast_to_state(&mut state, "12:00", "Me", "newest", "m-last".into(), 99.0);
    assert_eq!(state.messages.len(), MAX_MESSAGE_HISTORY);
    assert!(
        state.messages[MAX_MESSAGE_HISTORY - 1]
            .text
            .contains("newest")
    );
}

#[test]
fn test_push_outgoing_broadcast_peer_id_is_none() {
    let mut state = test_app_state();
    push_outgoing_broadcast_to_state(&mut state, "12:00", "Me", "test", "m1".into(), 1.0);
    assert_eq!(state.messages[0].sender_peer_id, None);
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

// ── send_message ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_send_broadcast_updates_state_and_sends_command() {
    let mut state = test_app_state();
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);

    send_message(
        &mut state,
        &swarm_cmd_tx,
        "hello".to_string(),
        p2p_app::tui_tabs::TabContent::Chat,
    )
    .await;

    assert_eq!(state.messages.len(), 1);
    assert!(state.messages[0].text.contains("hello"));
    assert!(state.chat_input.lines().join("").is_empty());

    let cmd = swarm_cmd_rx.try_recv().unwrap();
    match cmd {
        p2p_app::SwarmCommand::Publish { content, .. } => assert_eq!(content, "hello"),
        _ => panic!("expected Publish"),
    }
}

#[tokio::test]
async fn test_send_dm_updates_state_and_sends_command() {
    let mut state = test_app_state();
    let peer_id = "peer-dm";
    state.dynamic_tabs.add_dm_tab(peer_id.to_string());
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);

    send_message(
        &mut state,
        &swarm_cmd_tx,
        "secret".to_string(),
        p2p_app::tui_tabs::TabContent::Direct(peer_id.to_string()),
    )
    .await;

    assert!(state.dm_messages.contains_key(peer_id));
    assert!(state.dm_messages[peer_id][0].contains("secret"));
    assert!(state.chat_input.lines().join("").is_empty());

    let cmd = swarm_cmd_rx.try_recv().unwrap();
    match cmd {
        p2p_app::SwarmCommand::SendDm {
            content,
            ref peer_id,
            ..
        } => {
            assert_eq!(content, "secret");
            assert_eq!(peer_id, "peer-dm");
        }
        _ => panic!("expected SendDm"),
    }
}

fn with_test_db(f: impl FnOnce(mpsc::Sender<p2p_app::SwarmCommand>)) {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db_str = db_path.to_str().unwrap().to_string();
    p2p_app::db::set_cached_db_url(&db_str);
    p2p_app::db::init_database().unwrap();
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    f(swarm_cmd_tx);

    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url_cache();
}

#[test]
fn test_send_broadcast_saves_to_db() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    with_test_db(|swarm_cmd_tx| {
        rt.block_on(async {
            let mut state = test_app_state();

            send_message(
                &mut state,
                &swarm_cmd_tx,
                "db test".to_string(),
                p2p_app::tui_tabs::TabContent::Chat,
            )
            .await;

            let msgs = p2p_app::load_messages("test-net", 10).unwrap();
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0].content, "db test");
        });
    });
}

#[test]
fn test_send_dm_saves_to_db() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    with_test_db(|swarm_cmd_tx| {
        rt.block_on(async {
            let mut state = test_app_state();
            state.dynamic_tabs.add_dm_tab("dm-peer".to_string());

            send_message(
                &mut state,
                &swarm_cmd_tx,
                "dm db".to_string(),
                p2p_app::tui_tabs::TabContent::Direct("dm-peer".to_string()),
            )
            .await;

            let msgs = p2p_app::load_direct_messages("dm-peer", 10).unwrap();
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0].content, "dm db");
        });
    });
}
