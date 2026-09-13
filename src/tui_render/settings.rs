//! Settings-tab rendering (mirrors Flutter's `_Settings` screen).

use crate::tui_render_state::TuiRenderState;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Format an optional epoch-seconds timestamp as a readable local datetime.
#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
fn format_lost_at(ts: Option<f64>) -> String {
    ts.and_then(|t| {
        chrono::DateTime::from_timestamp(t as i64, 0)
            .map(|dt| crate::format_peer_datetime(dt.with_timezone(&chrono::Local).naive_local()))
    })
    .unwrap_or_else(|| "—".to_string())
}

/// The most recent connection we had to report on the Settings tab: this
/// session's last disconnect if any, otherwise the known peer with the newest
/// `last_seen`. Returns `(peer_id, formatted_timestamp)`.
fn last_connected_info(state: &TuiRenderState) -> Option<(String, String)> {
    if let (Some(peer_id), Some(at)) = (&state.last_connection_peer, state.last_connection_lost) {
        return Some((peer_id.clone(), format_lost_at(Some(at))));
    }
    state
        .peers
        .iter()
        .max_by(|a, b| a.last_seen.cmp(&b.last_seen))
        .map(|p| (p.peer_id.clone(), p.last_seen.clone()))
}

/// Render the Settings tab (mirrors the Flutter `_Settings` screen).
pub fn render_settings_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let connected = state.peer_count;
    let node_status = if state.node_running {
        "Running"
    } else {
        "Stopped"
    };

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "Nickname: {}  (press 'n' to edit)",
        state.own_nickname
    ));
    lines.push(format!("Peer ID: {}", state.local_peer_id));
    lines.push(format!("Database: {}", state.db_url));
    lines.push(format!("Platform: {}", state.platform));
    lines.push(format!("Node: {node_status}"));
    lines.push(format!("Network name: {}", crate::CHAT_TOPIC));
    lines.push(format!("Network size: {}", state.network_size));
    lines.push(format!("Connected peers: {connected}"));
    if state.connected_peer_ids.is_empty() {
        // Only ever show "currently connected" peers. When none are connected,
        // say when we last were (this session's record, else the most recently
        // seen known peer) instead of listing stale peers as connected.
        match last_connected_info(state) {
            Some((peer_id, ts)) => {
                let display = crate::get_peer_display_name(&peer_id)
                    .unwrap_or_else(|_| crate::fmt::short_peer_id(&peer_id));
                lines.push(format!("  Last connected {ts} to peer {display}"));
            }
            None => lines.push("  —".to_string()),
        }
    } else {
        for peer_id in &state.connected_peer_ids {
            let display = crate::get_peer_display_name(peer_id)
                .unwrap_or_else(|_| crate::fmt::short_peer_id(peer_id));
            lines.push(format!("  {display}"));
        }
    }
    lines.push("Listen addresses:".to_string());
    if state.listen_addrs.is_empty() {
        lines.push("  —".to_string());
    } else {
        for addr in &state.listen_addrs {
            lines.push(format!("  {addr}"));
        }
    }

    let para = Paragraph::new(lines.join("\n"))
        .block(Block::default().title("Settings").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}
