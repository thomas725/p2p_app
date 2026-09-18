//! Peer-info tab rendering (mirrors Flutter's `PeerInfoScreen`).

use crate::tui_render_state::TuiRenderState;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Render the Peer Info tab (mirrors the Flutter `PeerInfoScreen`).
pub fn render_peer_info_content(
    f: &mut ratatui::Frame,
    area: Rect,
    peer_id: &str,
    state: &TuiRenderState,
) {
    let display = crate::get_peer_display_name_or_short(peer_id);
    let local = state.local_nicknames.get(peer_id);
    let received = state.received_nicknames.get(peer_id);

    let (origin, detail) = if local.is_some() {
        ("Local nickname", "You set this nickname for the peer.")
    } else if received.is_some() {
        (
            "Received nickname",
            "Announced by the peer. (Receipt time is not tracked.)",
        )
    } else {
        (
            "Generated petname",
            "No nickname was known, so a petname was assigned locally.",
        )
    };

    let (first_seen, last_seen) = state
        .peers
        .iter()
        .find(|p| p.peer_id == peer_id)
        .map_or_else(
            || ("unknown".to_string(), "unknown".to_string()),
            |p| (p.first_seen.clone(), p.last_seen.clone()),
        );

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("Peer: {display}"));
    lines.push(format!("Peer ID: {peer_id}"));
    lines.push(format!("Origin: {origin}"));
    lines.push(format!("  {detail}"));
    if let Some(n) = local {
        lines.push(format!("Local nickname: {n}"));
    }
    if let Some(n) = received {
        lines.push(format!("Received nickname: {n}"));
    }
    lines.push(format!("First seen: {first_seen}"));
    lines.push(format!("Last seen: {last_seen}"));
    lines.push("Press Enter to open direct message".to_string());

    let para = Paragraph::new(lines.join("\n"))
        .block(Block::default().title("Peer Info").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}
