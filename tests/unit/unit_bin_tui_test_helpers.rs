use crate::tui::state::AppState;
use p2p_app::{DisplayMessage, PeerRecord};
use std::collections::{HashMap, VecDeque};

pub fn test_app_state() -> AppState {
    AppState::new(
        "test-net".to_string(),
        "TestUser".to_string(),
        "test-peer-id-1234".to_string(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        crate::tui::state::InitialStateData::default(),
    )
}

pub fn app_state_with_chat_messages(count: usize) -> AppState {
    let mut state = test_app_state();
    for i in 0..count {
        state.messages.push_back(DisplayMessage {
            text: format!("Message {i}"),
            sender_peer_id: Some(format!("peer{i}")),
        });
        state.message_ids.push_back(Some(format!("msg-{i}")));
    }
    state
}

pub fn app_state_with_peers(count: usize) -> AppState {
    let mut state = test_app_state();
    for i in 0..count {
        let id = format!("peer{i}");
        state.peers.push_back(PeerRecord {
            peer_id: id,
            first_seen: "2024-01-01 12:00:00".into(),
            last_seen: "2024-01-01 12:00:00".into(),
        });
    }
    // Mirror the production peer-add path (`command_processor` pushes to the
    // live deque then calls `resort_peers`), so `contacts` is a faithful view
    // and the Contacts tab tests see the same sort the app does.
    state.resort_peers();
    state
}

pub fn app_state_with_dm_messages(peer_id: &str, count: usize) -> AppState {
    let mut state = test_app_state();
    state.dynamic_tabs.add_dm_tab(peer_id.to_string());
    let msgs: VecDeque<String> = (0..count).map(|i| format!("DM message {i}")).collect();
    state.dm_messages.insert(peer_id.to_string(), msgs);
    state.dm_message_ids.insert(
        peer_id.to_string(),
        (0..count).map(|i| Some(format!("dm-{i}"))).collect(),
    );
    state.dm_scroll_state.insert(peer_id.to_string(), (0, true));
    state
        .dm_broadcast_scroll_state
        .insert(peer_id.to_string(), (0, true));
    state
}
