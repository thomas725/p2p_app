#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::panic_in_result_fn, clippy::unreachable, clippy::todo, clippy::unimplemented)]
#![allow(clippy::used_underscore_binding, clippy::significant_drop_tightening, clippy::match_wildcard_for_single_variants)]
use super::*;
use p2p_app::tui_tabs::TabContent;

#[test]
fn test_shortcuts_peers_tab_documents_dm_and_info() {
    let text = shortcuts_text(&TabContent::Peers, false);
    assert!(text.contains("Enter: open DM"));
    assert!(text.contains("i: Peer Info"));
    assert!(!text.contains("Ctrl+I: Peer Info"));
}

#[test]
fn test_shortcuts_direct_tab_documents_ctrl_i_on_kitty() {
    let text = shortcuts_text(&TabContent::Direct("peer-1".to_string()), true);
    assert!(text.contains("Ctrl+I: Peer Info"));
    assert!(!text.contains("| i: Peer Info"));
    assert!(!text.contains("Ctrl+?"));
}

#[test]
fn test_shortcuts_direct_tab_documents_ctrl_question_on_nonkitty() {
    let text = shortcuts_text(&TabContent::Direct("peer-1".to_string()), false);
    assert!(text.contains("Ctrl+?: Peer Info"));
    assert!(!text.contains("Ctrl+I: Peer Info"));
}

#[test]
fn test_shortcuts_plain_tabs_have_no_peer_info_hint() {
    for tab in [
        TabContent::Chat,
        TabContent::Log,
        TabContent::Settings,
        TabContent::PeerInfo("peer-1".to_string()),
    ] {
        assert!(!shortcuts_text(&tab, false).contains("Peer Info"));
        assert!(!shortcuts_text(&tab, true).contains("Peer Info"));
    }
}
