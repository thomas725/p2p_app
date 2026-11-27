#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]

use super::render_input_section;
use crate::tui::test_helpers::{app_state_with_dm_messages, test_app_state};
use p2p_app::tui_tabs::TabContent;
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

/// The live (bin) input renderer must show a Contacts search box — the library
/// renderer already did, but the running TUI used this separate function and
/// silently dropped the query on the floor (no title, no textarea).
#[test]
fn test_live_input_section_shows_contacts_search_title_and_query() {
    let mut state = test_app_state();
    state.set_contact_search("ali");
    state.begin_contact_search();
    assert!(state.is_searching_contacts());

    let tab = TabContent::Contacts;
    assert!(
        !tab.is_input_enabled(),
        "Contacts has no ordinary chat input"
    );

    let mut terminal = Terminal::new(TestBackend::new(80, 5)).unwrap();
    terminal
        .draw(|f| render_input_section(f, Rect::new(0, 0, 80, 5), &state, &tab))
        .unwrap();
    let text = buffer_text(&terminal);
    assert!(
        text.contains("Search Contacts"),
        "missing search title in: {text}"
    );
    assert!(text.contains("ali"), "missing live query in: {text}");
}

/// An armed reply must be visible before it is sent. The library renderer showed
/// the quote, but the live binary renderer used to print a plain `Input` title,
/// leaving the user with no confirmation of what `Ctrl+R` captured.
#[test]
fn test_live_input_section_shows_armed_reply_preview() {
    let mut state = app_state_with_dm_messages("reply-peer", 1);
    state.dm_messages.insert(
        "reply-peer".to_string(),
        std::collections::VecDeque::from(vec!["12:00 [Alice] hello from Alice".to_string()]),
    );
    state.dm_message_ids.insert(
        "reply-peer".to_string(),
        std::collections::VecDeque::from(vec![Some("dm-original".to_string())]),
    );
    let tab = TabContent::Direct("reply-peer".to_string());
    assert!(state.begin_reply(&tab).is_some(), "reply captured");

    let mut terminal = Terminal::new(TestBackend::new(80, 5)).unwrap();
    terminal
        .draw(|f| render_input_section(f, Rect::new(0, 0, 80, 5), &state, &tab))
        .unwrap();
    let text = buffer_text(&terminal);
    assert!(
        text.contains("Reply: ↳ Alice: hello from Alice"),
        "missing reply quote in: {text}"
    );
}

/// Outside search mode the Contacts tab keeps the plain input-block chrome —
/// the special search title/textarea must not leak into ordinary rendering.
#[test]
fn test_live_input_section_contacts_without_search_has_plain_title() {
    let state = test_app_state();
    let tab = TabContent::Contacts;
    let mut terminal = Terminal::new(TestBackend::new(80, 5)).unwrap();
    terminal
        .draw(|f| render_input_section(f, Rect::new(0, 0, 80, 5), &state, &tab))
        .unwrap();
    let text = buffer_text(&terminal);
    assert!(
        !text.contains("Search Contacts"),
        "search title leaked into plain Contacts render: {text}"
    );
}
