use crate::tui::state::AppState;
use std::collections::{HashMap, VecDeque};

pub fn test_app_state() -> AppState {
    AppState::new(
        "test-net".to_string(),
        "TestUser".to_string(),
        "test-peer-id-1234".to_string(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        VecDeque::new(),
        VecDeque::new(),
        HashMap::new(),
        VecDeque::new(),
        HashMap::new(),
        HashMap::new(),
    )
}

pub fn app_state_with_chat_messages(count: usize) -> AppState {
    let mut state = test_app_state();
    for i in 0..count {
        state
            .messages
            .push_back((format!("Message {i}"), Some(format!("peer{i}"))));
        state.message_ids.push_back(Some(format!("msg-{i}")));
    }
    state.visible_message_count = 5;
    state
}

pub fn app_state_with_peers(count: usize) -> AppState {
    let mut state = test_app_state();
    for i in 0..count {
        let id = format!("peer{i}");
        state.peers.push_back((
            id,
            "2024-01-01 12:00:00".into(),
            "2024-01-01 12:00:00".into(),
        ));
    }
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
