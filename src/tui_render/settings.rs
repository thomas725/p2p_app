//! Settings-tab rendering (mirrors Flutter's `_Settings` screen).

use crate::tui_render_state::TuiRenderState;
use num_traits::ToPrimitive;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Format an optional epoch-seconds timestamp as a readable local datetime.
fn format_lost_at(ts: Option<f64>) -> String {
    ts.and_then(|t| {
        chrono::DateTime::from_timestamp(t.to_i64().unwrap_or_default(), 0)
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

/// Append the network-diagnostics block (interfaces, reachability, DHT/relay).
fn render_network_details(lines: &mut Vec<String>, net: &crate::net_diag::NetworkDetails) {
    lines.push("Network details:".to_string());
    lines.push(format!("  Reachability: {}", net.reachability));

    let mut nat = net.nat_status.clone();
    if let Some(addr) = &net.nat_public_addr {
        nat.push_str(" (");
        nat.push_str(addr);
        nat.push(')');
    }
    lines.push(format!(
        "  AutoNAT: {nat} — probes {} ok / {} failed",
        net.nat_probe_successes, net.nat_probe_failures
    ));
    lines.push(format!(
        "  Hole punching: {}/{} attempts succeeded",
        net.hole_punch_successes, net.hole_punch_attempts
    ));
    let relay = if net.relay_reservations.is_empty() {
        net.relay_status.clone()
    } else {
        format!(
            "{} ({})",
            net.relay_status,
            net.relay_reservations.join(", ")
        )
    };
    lines.push(format!("  Relay: {relay}"));

    let dht_state = if net.dht_connected {
        "connected"
    } else {
        "not connected"
    };
    lines.push(format!(
        "  DHT: {dht_state} ({}, {} routing peers; bootstrap {} ok / {} failed)",
        net.dht_mode,
        net.dht_routing_peers,
        net.dht_bootstrap_successes,
        net.dht_bootstrap_failures
    ));
    lines.push(format!("  DHT announcement: {}", net.dht_announce_status));

    if !net.external_addrs.is_empty() {
        lines.push("  External addresses:".to_string());
        for addr in &net.external_addrs {
            lines.push(format!("    {addr}"));
        }
    }

    if !net.listen_addrs.is_empty() {
        lines.push("  Listen addresses:".to_string());
        for addr in &net.listen_addrs {
            lines.push(format!("    {addr}"));
        }
    }

    lines.push("  Interfaces:".to_string());
    if net.interfaces.is_empty() {
        lines.push("    —".to_string());
    }
    for iface in &net.interfaces {
        let mac = iface.mac.as_deref().unwrap_or("no MAC");
        let loopback = if iface.internal { " (loopback)" } else { "" };
        lines.push(format!("    {} — {mac}{loopback}", iface.name));
        if !iface.ipv4.is_empty() {
            lines.push(format!("      IPv4: {}", iface.ipv4.join(", ")));
        }
        if !iface.ipv6.is_empty() {
            lines.push(format!("      IPv6: {}", iface.ipv6.join(", ")));
        }
    }
}

/// Render the Settings tab (mirrors the Flutter `_Settings` screen).
pub fn render_settings_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let connected = state.peer_count;
    let node_status = if state.runtime.node_running {
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
    match &state.own_user_id {
        Some(user_id) => lines.push(format!("User ID: {user_id}")),
        None => lines.push("User ID: —".to_string()),
    }
    if !state.own_user_devices.is_empty() {
        lines.push(format!("Bound devices: {}", state.own_user_devices.len()));
        for (device_peer, bound_at) in &state.own_user_devices {
            let display = crate::get_peer_display_name_or_short(device_peer);
            lines.push(format!("  {display} (bound {bound_at})"));
        }
    }
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
                let display = crate::get_peer_display_name_or_short(&peer_id);
                lines.push(format!("  Last connected {ts} to peer {display}"));
            }
            None => lines.push("  —".to_string()),
        }
    } else {
        for peer_id in &state.connected_peer_ids {
            let display = crate::get_peer_display_name_or_short(peer_id);
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

    render_network_details(&mut lines, &state.network);

    let para = Paragraph::new(lines.join("\n"))
        .block(Block::default().title("Settings").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}
