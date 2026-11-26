//! Settings-tab rendering (mirrors Flutter's `_Settings` screen).

use crate::tui_render_state::TuiRenderState;
use num_traits::ToPrimitive;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};
use std::collections::HashSet;
use std::hash::BuildHasher;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// One visual Settings row and the peer it opens, if any.
///
/// Rendering and mouse hit-testing both consume [`settings_visual_lines`], so a
/// wrapped peer row stays clickable on every line it occupies. Rows without a
/// `peer_id` are informational and ignore clicks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsLine {
    /// Exact text rendered on this visual row.
    pub text: String,
    /// Peer whose Peer Info this row opens.
    pub peer_id: Option<String>,
}

impl SettingsLine {
    /// A plain informational row.
    #[must_use]
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            peer_id: None,
        }
    }

    /// A peer row, marked with Flutter's `›` affordance.
    #[must_use]
    pub fn peer(text: impl Into<String>, peer_id: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            peer_id: Some(peer_id.into()),
        }
    }
}

/// Bound devices in deterministic peer-ID order.
#[must_use]
pub fn sorted_settings_device_ids(devices: &[(String, String)]) -> Vec<String> {
    let mut ids: Vec<String> = devices.iter().map(|(peer_id, _)| peer_id.clone()).collect();
    ids.sort();
    ids
}

/// Currently connected peers in deterministic peer-ID order.
#[must_use]
pub fn sorted_settings_peer_ids<S: BuildHasher>(connected: &HashSet<String, S>) -> Vec<String> {
    let mut ids: Vec<String> = connected.iter().cloned().collect();
    ids.sort();
    ids
}

/// The peer `Ctrl+P` opens on Settings: first connected peer, else first bound
/// device. Both lists are sorted by peer ID so the result never depends on
/// `HashSet` iteration order or DB row order.
#[must_use]
pub fn settings_peer_target<S: BuildHasher>(
    connected: &HashSet<String, S>,
    devices: &[(String, String)],
) -> Option<String> {
    sorted_settings_peer_ids(connected)
        .into_iter()
        .next()
        .or_else(|| sorted_settings_device_ids(devices).into_iter().next())
}

/// Format an optional epoch-seconds timestamp as a readable local datetime.
fn format_lost_at(ts: Option<f64>) -> String {
    ts.and_then(|t| {
        chrono::DateTime::from_timestamp(t.to_i64().unwrap_or_default(), 0)
            .map(|dt| crate::format_peer_datetime(dt.with_timezone(&chrono::Local).naive_local()))
    })
    .unwrap_or_else(|| "—".to_string())
}

/// Append one word to a wrapped row, splitting overlong words by display width.
///
/// `has_word` distinguishes the preserved indentation prefix from real content.
/// The indent survives onto the first row whenever at least one character of
/// content fits beside it; otherwise it is dropped rather than emitted alone.
fn push_word(
    word: &str,
    width: usize,
    rows: &mut Vec<SettingsLine>,
    current: &mut String,
    current_width: &mut usize,
    has_word: &mut bool,
    peer_id: Option<&String>,
) {
    let word_width = word.width();
    let separator = usize::from(*has_word);
    if current_width
        .saturating_add(separator)
        .saturating_add(word_width)
        <= width
    {
        if *has_word {
            current.push(' ');
            *current_width = current_width.saturating_add(1);
        }
        current.push_str(word);
        *current_width = current_width.saturating_add(word_width);
        *has_word = true;
        return;
    }
    if *has_word {
        rows.push(SettingsLine {
            text: std::mem::take(current),
            peer_id: peer_id.cloned(),
        });
        *current_width = 0;
        *has_word = false;
    }
    // A word that fits on an indent-only row keeps that indent; an overlong
    // word is split by display width from here instead.
    if current_width.saturating_add(word_width) <= width {
        current.push_str(word);
        *current_width = current_width.saturating_add(word_width);
        *has_word = true;
        return;
    }
    for character in word.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or_default();
        if current_width.saturating_add(character_width) > width {
            if *has_word {
                rows.push(SettingsLine {
                    text: std::mem::take(current),
                    peer_id: peer_id.cloned(),
                });
            }
            current.clear();
            *current_width = 0;
            *has_word = false;
        }
        current.push(character);
        *current_width = current_width.saturating_add(character_width);
        *has_word = true;
    }
}

/// Greedy word wrap used by both the renderer and the click handler.
///
/// Ratatui's `WordWrapper` is private, so Settings owns a small deterministic
/// wrapper instead: words break on spaces, overlong words break by display
/// width, leading indentation stays on the first row, and a peer target survives
/// onto every continuation row.
fn wrap_settings_line(line: SettingsLine, width: usize) -> Vec<SettingsLine> {
    if width == 0 {
        return vec![line];
    }
    let indent: String = line
        .text
        .chars()
        .take_while(|character| character.is_whitespace())
        .collect();
    let body = line.text.trim_start_matches(char::is_whitespace);
    // Keep the `› ` affordance with the first wrapped line, just like leading
    // whitespace. Continuation rows keep only the peer target, so the marker is
    // never repeated on every line of a long name.
    let (prefix, body) = if line.peer_id.is_some() {
        match body.strip_prefix("› ") {
            Some(rest) => (format!("{indent}› "), rest),
            None => (indent, body),
        }
    } else {
        (indent, body)
    };
    let mut rows: Vec<SettingsLine> = Vec::new();
    let mut current = prefix;
    let mut current_width = current.width();
    let mut has_word = false;
    for word in body.split_whitespace() {
        push_word(
            word,
            width,
            &mut rows,
            &mut current,
            &mut current_width,
            &mut has_word,
            line.peer_id.as_ref(),
        );
    }
    if has_word || !current.is_empty() {
        rows.push(SettingsLine {
            text: current,
            peer_id: line.peer_id.clone(),
        });
    }
    if rows.is_empty() {
        rows.push(SettingsLine {
            text: String::new(),
            peer_id: line.peer_id,
        });
    }
    rows
}

/// Append the persisted bootstrap/relay list (US-050 follow-up 1).
///
/// The highlight moves with Up/Down/Home/End on the Settings tab; `b`/`r`
/// open the add prompt for a bootstrap/relay multiaddr and `x` removes the
/// highlighted entry. The node only reads the list at start, so the header
/// says so.
fn render_network_peers(lines: &mut Vec<SettingsLine>, state: &TuiRenderState) {
    lines.push(SettingsLine::plain(format!(
        "Network peers ({}) — applies on next node start:",
        state.network_peers.len()
    )));
    if state.network_peers.is_empty() {
        lines.push(SettingsLine::plain("  — none configured"));
    } else {
        for (index, peer) in state.network_peers.iter().enumerate() {
            let marker = if index == state.network_peer_selection {
                ">"
            } else {
                " "
            };
            lines.push(SettingsLine::plain(format!(
                "{marker} {} {}",
                peer.kind, peer.multiaddr
            )));
        }
    }
    lines.push(SettingsLine::plain(
        "  b: add bootstrap | r: add relay | x: remove | Up/Down: select",
    ));
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
fn render_network_details(lines: &mut Vec<SettingsLine>, net: &crate::net_diag::NetworkDetails) {
    lines.push(SettingsLine::plain("Network details:"));
    lines.push(SettingsLine::plain(format!(
        "  Reachability: {}",
        net.reachability
    )));

    let mut nat = net.nat_status.clone();
    if let Some(addr) = &net.nat_public_addr {
        nat.push_str(" (");
        nat.push_str(addr);
        nat.push(')');
    }
    lines.push(SettingsLine::plain(format!(
        "  AutoNAT: {nat} — probes {} ok / {} failed",
        net.nat_probe_successes, net.nat_probe_failures
    )));
    lines.push(SettingsLine::plain(format!(
        "  Hole punching: {}/{} attempts succeeded",
        net.hole_punch_successes, net.hole_punch_attempts
    )));
    let relay = if net.relay_reservations.is_empty() {
        net.relay_status.clone()
    } else {
        format!(
            "{} ({})",
            net.relay_status,
            net.relay_reservations.join(", ")
        )
    };
    lines.push(SettingsLine::plain(format!("  Relay: {relay}")));

    let dht_state = if net.dht_connected {
        "connected"
    } else {
        "not connected"
    };
    lines.push(SettingsLine::plain(format!(
        "  DHT: {dht_state} ({}, {} routing peers; bootstrap {} ok / {} failed)",
        net.dht_mode,
        net.dht_routing_peers,
        net.dht_bootstrap_successes,
        net.dht_bootstrap_failures
    )));
    // Mirror Flutter: surface the announced flag, not only the status line.
    let announcement = if net.dht_announced {
        format!("Announced ({})", net.dht_announce_status)
    } else {
        net.dht_announce_status.clone()
    };
    lines.push(SettingsLine::plain(format!(
        "  DHT announcement: {announcement}"
    )));

    if !net.external_addrs.is_empty() {
        lines.push(SettingsLine::plain("  External addresses:"));
        for addr in &net.external_addrs {
            lines.push(SettingsLine::plain(format!("    {addr}")));
        }
    }

    lines.push(SettingsLine::plain("  Listen addresses:"));
    if net.listen_addrs.is_empty() {
        lines.push(SettingsLine::plain("    —"));
    } else {
        for addr in &net.listen_addrs {
            lines.push(SettingsLine::plain(format!("    {addr}")));
        }
    }

    lines.push(SettingsLine::plain("  Interfaces:"));
    if net.interfaces.is_empty() {
        lines.push(SettingsLine::plain("    —"));
    }
    for iface in &net.interfaces {
        let mac = iface.mac.as_deref().unwrap_or("no MAC");
        let loopback = if iface.internal { " (loopback)" } else { "" };
        lines.push(SettingsLine::plain(format!(
            "    {} — {mac}{loopback}",
            iface.name
        )));
        if !iface.ipv4.is_empty() {
            lines.push(SettingsLine::plain(format!(
                "      IPv4: {}",
                iface.ipv4.join(", ")
            )));
        }
        if !iface.ipv6.is_empty() {
            lines.push(SettingsLine::plain(format!(
                "      IPv6: {}",
                iface.ipv6.join(", ")
            )));
        }
    }
}

/// Logical Settings rows, before wrapping to the terminal width.
fn settings_logical_lines(state: &TuiRenderState) -> Vec<SettingsLine> {
    let connected = state.peer_count;
    let node_status = if state.runtime.node_running {
        "Running"
    } else {
        "Stopped"
    };

    let mut lines: Vec<SettingsLine> = Vec::new();
    lines.push(SettingsLine::plain(format!(
        "Nickname: {}  (press 'n' to edit)",
        state.own_nickname
    )));
    lines.push(SettingsLine::plain(format!(
        "Peer ID: {}",
        state.local_peer_id
    )));
    match &state.own_user_id {
        Some(user_id) => lines.push(SettingsLine::plain(format!("User ID: {user_id}"))),
        None => lines.push(SettingsLine::plain("User ID: —")),
    }
    if !state.own_user_devices.is_empty() {
        lines.push(SettingsLine::plain(format!(
            "Bound devices: {}",
            state.own_user_devices.len()
        )));
        let mut devices = state.own_user_devices.clone();
        devices.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for (device_peer, bound_at) in devices {
            let display = crate::get_peer_display_name_or_short(&device_peer);
            lines.push(SettingsLine::peer(
                format!("› {display} (bound {bound_at})"),
                device_peer,
            ));
        }
    }
    render_network_peers(&mut lines, state);
    let lan_only = crate::settings::lan_only();
    lines.push(SettingsLine::plain(format!(
        "LAN only: {} (press 'l' to toggle; applies on next node start)",
        if lan_only { "yes" } else { "no" }
    )));
    lines.push(SettingsLine::plain(format!(
        "Theme: {} / {} (press 't' for dark/light, 'c' for accent color)",
        if state.palette.dark { "dark" } else { "light" },
        state.palette.accent.as_str()
    )));
    let notification_settings = crate::notifications::settings();
    lines.push(SettingsLine::plain(format!(
        "Notification sound: {} (press 's' to toggle)",
        if notification_settings.sound {
            "yes"
        } else {
            "no"
        }
    )));
    lines.push(SettingsLine::plain(format!(
        "Desktop notifications: {} (press 'o' to toggle)",
        if notification_settings.desktop {
            "yes"
        } else {
            "no"
        }
    )));
    lines.push(SettingsLine::plain(format!("Database: {}", state.db_url)));
    lines.push(SettingsLine::plain(format!("Platform: {}", state.platform)));
    lines.push(SettingsLine::plain(format!("Node: {node_status}")));
    lines.push(SettingsLine::plain(format!(
        "Network name: {}",
        crate::CHAT_TOPIC
    )));
    lines.push(SettingsLine::plain(format!(
        "Network size: {}",
        state.network_size
    )));
    lines.push(SettingsLine::plain(format!("Connected peers: {connected}")));
    if state.connected_peer_ids.is_empty() {
        // Only ever show "currently connected" peers. When none are connected,
        // say when we last were (this session's record, else the most recently
        // seen known peer) instead of listing stale peers as connected.
        match last_connected_info(state) {
            Some((peer_id, ts)) => {
                let display = crate::get_peer_display_name_or_short(&peer_id);
                lines.push(SettingsLine::plain(format!(
                    "  Last connected {ts} to peer {display}"
                )));
            }
            None => lines.push(SettingsLine::plain("  —")),
        }
    } else {
        let connected_peers = sorted_settings_peer_ids(&state.connected_peer_ids);
        for peer_id in connected_peers {
            let display = crate::get_peer_display_name_or_short(&peer_id);
            lines.push(SettingsLine::peer(format!("› {display}"), peer_id));
        }
    }
    render_network_details(&mut lines, &state.network);
    lines
}

/// Exact visual rows for the Settings pane at the given inner width.
#[must_use]
pub fn settings_visual_lines(state: &TuiRenderState, width: usize) -> Vec<SettingsLine> {
    settings_logical_lines(state)
        .into_iter()
        .flat_map(|line| wrap_settings_line(line, width))
        .collect()
}

/// Render the Settings tab (mirrors the Flutter `_Settings` screen).
pub fn render_settings_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let width = usize::from(area.width.saturating_sub(2));
    let text = settings_visual_lines(state, width)
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>()
        .join("\n");
    let para = Paragraph::new(text).block(Block::default().title("Settings").borders(Borders::ALL));
    f.render_widget(para, area);
}
