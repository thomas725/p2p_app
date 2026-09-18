use super::*;
use crate::tui_tabs::TabContent;

#[test]
fn test_tui_render_state_default() {
    let state = TuiRenderState::default();
    assert_eq!(state.active_tab, 0);
    assert_eq!(state.tab_titles.len(), 5);
    assert!(state.messages.is_empty());
    assert!(state.peers.is_empty());
    assert!(state.log_messages.is_empty());
}

#[test]
fn test_tui_render_state_new() {
    let state = TuiRenderState::new();
    assert!(state.connected);
    assert!(!state.mouse_capture);
    assert!(state.popup.is_none());
    assert!(state.log_auto_scroll);
    assert_eq!(state.log_scroll_offset, 0);
}

#[test]
fn test_tui_render_state_with_sample_data() {
    let state = TuiRenderState::with_sample_data();
    assert_eq!(state.messages.len(), 3);
    assert_eq!(state.peers.len(), 2);
    assert_eq!(state.peer_count, 2);
}

#[test]
fn test_tui_render_state_add_message() {
    let mut state = TuiRenderState::new();
    state.add_message("Hello");
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0], "Hello");
}

#[test]
fn test_tui_render_state_add_peer() {
    let mut state = TuiRenderState::new();
    state.add_peer("peer1", "Alice", "Online");
    assert_eq!(state.peers.len(), 1);
    assert_eq!(state.peers[0].peer_id, "peer1");
    assert_eq!(state.peers[0].first_seen, "Alice");
}

#[test]
fn test_tui_render_state_add_dm_message() {
    let mut state = TuiRenderState::new();
    state.add_dm_message("peer1", "Hello there");
    assert!(state.dm_messages.contains_key("peer1"));
    assert_eq!(state.dm_messages["peer1"].len(), 1);
}

#[test]
fn test_get_tab_content_chat() {
    let mut state = TuiRenderState::new();
    state.active_tab = 0;
    assert_eq!(get_tab_content(&state), TabContent::Chat);
}

#[test]
fn test_get_tab_content_peers() {
    let mut state = TuiRenderState::new();
    state.active_tab = 1;
    assert_eq!(get_tab_content(&state), TabContent::Peers);
}

#[test]
fn test_get_tab_content_log() {
    let mut state = TuiRenderState::new();
    state.active_tab = 3;
    assert_eq!(get_tab_content(&state), TabContent::Log);
}

#[test]
fn test_get_tab_content_groups() {
    let mut state = TuiRenderState::new();
    state.active_tab = 2;
    assert_eq!(get_tab_content(&state), TabContent::Groups);
}

#[test]
fn test_get_tab_content_direct() {
    let mut state = TuiRenderState::new();
    state.tab_titles.push("DM: Alice".to_string());
    state.active_tab = state.tab_titles.len() - 1;
    if let TabContent::Direct(peer) = get_tab_content(&state) {
        assert_eq!(peer, "Alice");
    } else {
        panic!("expected Direct");
    }
}

#[test]
fn test_tui_tab_content_is_input_enabled() {
    assert!(TabContent::Chat.is_input_enabled());
    assert!(TabContent::Direct("peer".to_string()).is_input_enabled());
    assert!(!TabContent::Peers.is_input_enabled());
    assert!(!TabContent::Log.is_input_enabled());
}

#[test]
fn test_tui_render_state_clone() {
    let state = TuiRenderState::new();
    let cloned = state.clone();
    assert_eq!(cloned.active_tab, state.active_tab);
    assert_eq!(cloned.messages.len(), state.messages.len());
}

#[test]
fn test_count_lines_single_short_line() {
    assert_eq!(count_lines("hello", 80), 1);
}

#[test]
fn test_count_lines_wraps_long_line() {
    assert_eq!(count_lines("hello world foo bar baz quux", 10), 3);
}

#[test]
fn test_count_lines_empty_string() {
    assert_eq!(count_lines("", 80), 1);
}

#[test]
fn test_count_lines_newlines() {
    assert_eq!(count_lines("line1\nline2\nline3", 80), 3);
}

#[test]
fn test_calc_visible_strings_empty() {
    let msgs: VecDeque<String> = VecDeque::new();
    let (visible, offset) = calc_visible_strings(&msgs, true, 0, 80, 10);
    assert_eq!(visible, 0);
    assert_eq!(offset, 0);
}

#[test]
fn test_calc_visible_strings_auto_scroll_shows_all() {
    let msgs = VecDeque::from(vec!["a".to_string(), "b".to_string()]);
    let (visible, offset) = calc_visible_strings(&msgs, true, 0, 80, 10);
    assert_eq!(visible, 2);
    assert_eq!(offset, 0);
}

#[test]
fn test_calc_visible_strings_manual_scroll() {
    let msgs = VecDeque::from(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let (visible, offset) = calc_visible_strings(&msgs, false, 1, 80, 10);
    assert_eq!(visible, 2);
    assert_eq!(offset, 1);
}

#[test]
fn test_list_item_lines() {
    assert_eq!(list_item_lines("hello"), 1);
    assert_eq!(list_item_lines(""), 1);
    assert_eq!(list_item_lines("line1\nline2\nline3"), 3);
    // A long single line is still one List row (clipped, not width-wrapped).
    assert_eq!(list_item_lines("a".repeat(200).as_str()), 1);
}

#[test]
fn test_list_item_lines_differentiates_from_wrapped_count() {
    // `list_item_lines` counts newline segments only, while `count_lines`
    // also estimates width-wrapping — the two must diverge for a long line.
    let long = "hello world foo bar baz quux";
    assert_eq!(count_lines(long, 10), 3);
    assert_eq!(list_item_lines(long), 1);
}

#[test]
fn test_calc_visible_list_items_empty() {
    let msgs: VecDeque<String> = VecDeque::new();
    let (visible, offset) = calc_visible_list_items(&msgs, true, 0, 10);
    assert_eq!(visible, 0);
    assert_eq!(offset, 0);
}

#[test]
fn test_calc_visible_list_items_auto_scroll_shows_all() {
    let msgs = VecDeque::from(vec!["a".to_string(), "b".to_string()]);
    let (visible, offset) = calc_visible_list_items(&msgs, true, 0, 10);
    assert_eq!(visible, 2);
    assert_eq!(offset, 0);
}

#[test]
fn test_calc_visible_list_items_manual_scroll() {
    let msgs = VecDeque::from(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let (visible, offset) = calc_visible_list_items(&msgs, false, 1, 10);
    assert_eq!(visible, 2);
    assert_eq!(offset, 1);
}

#[test]
fn test_calc_visible_list_items_multiline_item_uses_row_count() {
    // A message spanning two newline segments occupies two List rows, so a
    // 3-row pane fits it plus one more single-row message.
    let msgs = VecDeque::from(vec![
        "top\nbottom".to_string(),
        "single".to_string(),
        "overflow".to_string(),
    ]);
    let (visible, offset) = calc_visible_list_items(&msgs, true, 0, 3);
    assert_eq!(visible, 2);
    // Auto-scroll from the bottom: the last two items fit.
    assert_eq!(offset, 1);
}

#[test]
fn test_calc_visible_list_items_long_line_is_one_row() {
    // Unlike calc_visible_strings (which width-wraps), a very long single-line
    // message occupies one List row, so all three fit in a 3-row pane.
    let long = "x".repeat(500);
    let msgs = VecDeque::from(vec![long, "y".to_string(), "z".to_string()]);
    let (visible, offset) = calc_visible_list_items(&msgs, true, 0, 3);
    assert_eq!(visible, 3);
    assert_eq!(offset, 0);
}

#[test]
fn test_broadcast_receipt_prefix_no_receipts() {
    let receipts = HashMap::new();
    assert_eq!(broadcast_receipt_prefix(Some("msg-1"), &receipts), "  ");
}

#[test]
fn test_broadcast_receipt_prefix_with_receipt() {
    let mut receipts = HashMap::new();
    receipts.insert(
        "msg-1".to_string(),
        HashMap::from([("peer-1".to_string(), 1.0)]),
    );
    assert_eq!(broadcast_receipt_prefix(Some("msg-1"), &receipts), "v ");
}

#[test]
fn test_broadcast_receipt_prefix_no_msg_id() {
    let receipts = HashMap::new();
    assert_eq!(broadcast_receipt_prefix(None, &receipts), "  ");
}

#[test]
fn test_dm_receipt_prefix_no_receipt() {
    let receipts = HashMap::new();
    assert_eq!(dm_receipt_prefix(Some("msg-1"), &receipts), "  ");
}

#[test]
fn test_dm_receipt_prefix_with_receipt() {
    let receipts = HashMap::from([("msg-1".to_string(), ("peer-1".to_string(), 1.0))]);
    assert_eq!(dm_receipt_prefix(Some("msg-1"), &receipts), "v ");
}

#[test]
fn test_tui_render_state_debug_format() {
    let state = TuiRenderState::new();
    let debug_str = format!("{state:?}");
    assert!(debug_str.contains("TuiRenderState"));
}

#[test]
fn test_tui_tab_content_clone() {
    let content = TabContent::Direct("peer1".to_string());
    let cloned = content.clone();
    assert_eq!(cloned, content);
}

#[test]
fn test_tui_tab_content_debug_format() {
    let content = TabContent::Chat;
    let debug_str = format!("{content:?}");
    assert!(debug_str.contains("Chat"));
}

#[test]
fn test_row_to_visible_index_before_first_content() {
    assert_eq!(row_to_visible_index(&[1, 1], 2, 1), None);
}

#[test]
fn test_row_to_visible_index_at_first_content_row() {
    assert_eq!(row_to_visible_index(&[1, 1], 2, 2), Some(0));
}

#[test]
fn test_row_to_visible_index_within_first_message() {
    let line_counts = vec![3, 1, 2];
    assert_eq!(row_to_visible_index(&line_counts, 0, 0), Some(0));
    assert_eq!(row_to_visible_index(&line_counts, 0, 1), Some(0));
    assert_eq!(row_to_visible_index(&line_counts, 0, 2), Some(0));
}

#[test]
fn test_row_to_visible_index_within_second_message() {
    let line_counts = vec![3, 1, 2];
    assert_eq!(row_to_visible_index(&line_counts, 0, 3), Some(1));
}

#[test]
fn test_row_to_visible_index_within_third_message() {
    let line_counts = vec![3, 1, 2];
    assert_eq!(row_to_visible_index(&line_counts, 0, 4), Some(2));
    assert_eq!(row_to_visible_index(&line_counts, 0, 5), Some(2));
}

#[test]
fn test_row_to_visible_index_beyond_all_messages() {
    let line_counts = vec![3, 1, 2];
    let total_lines: usize = line_counts.iter().sum();
    assert_eq!(row_to_visible_index(&line_counts, 0, total_lines), None);
}

#[test]
fn test_row_to_visible_index_empty() {
    assert_eq!(row_to_visible_index(&[], 0, 0), None);
}

#[test]
fn test_row_to_visible_index_first_content_row_nonzero() {
    let line_counts = vec![5, 3];
    assert_eq!(row_to_visible_index(&line_counts, 3, 3), Some(0));
    assert_eq!(row_to_visible_index(&line_counts, 3, 7), Some(0));
    assert_eq!(row_to_visible_index(&line_counts, 3, 8), Some(1));
    assert_eq!(row_to_visible_index(&line_counts, 3, 10), Some(1));
    assert_eq!(row_to_visible_index(&line_counts, 3, 11), None);
}

// ── max_list_scroll_offset ────────────────────────────────────────────────────

#[test]
fn test_max_list_scroll_offset_empty() {
    assert_eq!(max_list_scroll_offset(&[], 5), 0);
}

#[test]
fn test_max_list_scroll_offset_content_shorter_than_pane() {
    assert_eq!(max_list_scroll_offset(&[1, 1, 1], 5), 0);
}

#[test]
fn test_max_list_scroll_offset_content_exactly_fills_pane() {
    assert_eq!(max_list_scroll_offset(&[2, 2, 1], 5), 0);
}

#[test]
fn test_max_list_scroll_offset_content_exceeds_pane() {
    // 10 one-line messages in a 9-line pane: bottom is one row down.
    assert_eq!(max_list_scroll_offset(&[1; 10], 9), 1);
    // The offset counts whole items, not display lines: two 3-line messages
    // fill a 6-line pane, so the remaining two messages can be scrolled to.
    assert_eq!(max_list_scroll_offset(&[3, 3, 3, 3], 6), 2);
    // A message taller than the pane still counts as one visible item.
    assert_eq!(max_list_scroll_offset(&[10, 1], 4), 1);
}

#[test]
fn test_max_list_scroll_offset_zero_usable_height() {
    // Even with no room, one item is always kept visible.
    assert_eq!(max_list_scroll_offset(&[1, 1, 1, 1], 0), 3);
}
