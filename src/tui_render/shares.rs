//! Shares tab: known shares on the left, the highlighted share's grants,
//! manifest, connected peers and discovered offers on the right. Mirrors
//! Flutter's list → detail/discovery flow; grant editing uses the shared modal
//! input, while binding/snapshots stay native.

use crate::shares::{Share, ShareGrant, ShareKind, kind_label};
use crate::tui_helpers::peer_table_visible_range;
use crate::tui_render::bordered_block;
use crate::tui_render_state::TuiRenderState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
};

/// One compact row in the shares list.
fn share_row(share: &Share) -> String {
    format!(
        "{} [{}] rev {}",
        share.name,
        kind_label(share.kind),
        share.revision
    )
}

/// Detail pane lines for the selected share, plus the latest share-lane event.
fn detail_lines(share: Option<&Share>, state: &TuiRenderState) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(notice) = state.share_event_notice.as_deref() {
        lines.push(format!("Last event: {notice}"));
        lines.push(String::new());
    }
    lines.push("c: create a private share".to_string());
    lines.push("b: bind folder | s: snapshot | u: publish".to_string());
    if let Some(share) = share {
        let grants: &[ShareGrant] = state
            .share_grants
            .get(&share.share_id)
            .map_or(&[], Vec::as_slice);
        lines.push(format!("Name: {}", share.name));
        lines.push(format!("ID: {}", share.share_id));
        lines.push(format!(
            "Kind: {} (writes: {})",
            kind_label(share.kind),
            if share.public_write { "allowed" } else { "no" },
        ));
        lines.push(format!("Owner: {}", share.owner_user_id));
        lines.push(format!(
            "Local root: {}",
            share.local_root.as_deref().unwrap_or("not bound")
        ));
        lines.push(format!("Revision: {}", share.revision));
        if state
            .own_user_id
            .as_deref()
            .is_some_and(|id| id == share.owner_user_id)
        {
            lines.push("Access: p: toggle visibility | w: toggle public writes".to_string());
        }
        push_grant_lines(&mut lines, share, grants, state);
        push_manifest_lines(&mut lines, share, state);
    } else {
        lines.push("No shares known to this install.".to_string());
    }
    push_peer_lines(&mut lines, state);
    push_discovered_lines(&mut lines, state);
    lines
}

/// Private shares list their verified grants; public shares have no ACL rows,
/// mirroring Flutter's hidden Grants section.
fn push_grant_lines(
    lines: &mut Vec<String>,
    share: &Share,
    grants: &[ShareGrant],
    state: &TuiRenderState,
) {
    if share.kind != ShareKind::Private {
        return;
    }
    lines.push(format!("Grants ({}):", grants.len()));
    if grants.is_empty() {
        lines.push("  none".to_string());
    }
    for (index, grant) in grants.iter().enumerate() {
        let marker = if index == state.share_grant_selection {
            ">"
        } else {
            " "
        };
        lines.push(format!(
            " {marker} {} ({})",
            grant.user_id,
            crate::shares::mode_label(grant.mode)
        ));
    }
    lines.push("  a: add grant | x: revoke highlighted grant".to_string());
}

/// Show the cached manifest's files with a separate cursor and deletion marks,
/// plus the reload hint when the snapshot no longer matches the share revision.
fn push_manifest_lines(lines: &mut Vec<String>, share: &Share, state: &TuiRenderState) {
    let Some(manifest) = share.manifest.as_ref() else {
        lines.push("Files: no manifest yet".to_string());
        return;
    };
    lines.push(format!("Files ({}):", manifest.entries.len()));
    if !crate::shares::manifest_matches_share(share) {
        lines.push("  stale snapshot: r: reload before publishing".to_string());
    }
    let last = manifest.entries.len().saturating_sub(1);
    for (index, entry) in manifest.entries.iter().enumerate() {
        let marker = if index == state.share_manifest_selection.min(last) {
            ">"
        } else {
            " "
        };
        let delete = if state.share_deleted_paths.contains(&entry.path) {
            "x"
        } else {
            " "
        };
        lines.push(format!(
            " {marker} [{delete}] {} ({})",
            entry.path,
            crate::files::format_file_size(entry.size)
        ));
    }
    lines.push("  Ctrl+Left/Right: file | d: mark delete | Ctrl+D: publish".to_string());
}

/// Connected peers in canonical order, with the network-action keys.
fn push_peer_lines(lines: &mut Vec<String>, state: &TuiRenderState) {
    lines.push(String::new());
    lines.push(format!("Connected peers ({}):", state.share_peer_ids.len()));
    if state.share_peer_ids.is_empty() {
        lines.push("  none".to_string());
        return;
    }
    for (index, peer_id) in state.share_peer_ids.iter().enumerate() {
        let marker = if index
            == state
                .share_peer_selection
                .min(state.share_peer_ids.len().saturating_sub(1))
        {
            ">"
        } else {
            " "
        };
        lines.push(format!(" {marker} {peer_id}"));
    }
    lines.push("  Left/Right select | m: refresh manifest | o: re-advertise".to_string());
}

/// Offers advertised by connected peers, kept separate from accepted shares
/// until the user explicitly commits one through the canonical Rust store.
fn push_discovered_lines(lines: &mut Vec<String>, state: &TuiRenderState) {
    lines.push(String::new());
    lines.push(format!(
        "Discovered offers ({}):",
        state.discovered_shares.len()
    ));
    if state.discovered_shares.is_empty() {
        lines.push("  none | f: ask selected peer for shares".to_string());
        return;
    }
    let last = state.discovered_shares.len().saturating_sub(1);
    for (index, discovered) in state.discovered_shares.iter().enumerate() {
        let marker = if index == state.share_discovery_selection.min(last) {
            ">"
        } else {
            " "
        };
        let offer = &discovered.offer;
        lines.push(format!(
            " {marker} {} [{}] rev {} from {}",
            offer.name,
            kind_label(offer.kind),
            offer.revision,
            discovered.peer_id
        ));
        lines.push(format!(
            "     id {} | claimed owner {} (unverified)",
            offer.share_id,
            offer.owner_user_id.as_deref().unwrap_or("none")
        ));
    }
    lines.push("  j/k: offer | Enter: accept | f: discover".to_string());
}

/// Render the Shares tab: a selectable share list and the selected share's
/// metadata, grants summary, manifest and discovery panes.
pub fn render_shares_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let total = state.shares.len();
    let selected = (total > 0).then(|| state.share_selection.min(total.saturating_sub(1)));

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);
    let list_area = panes.first().copied().unwrap_or(area);
    let detail_area = panes.get(1).copied().unwrap_or(area);

    // The list block loses two rows to borders.
    let page_height = usize::from(list_area.height.saturating_sub(2)).max(1);
    let (start, end) = peer_table_visible_range(0, selected, total, page_height);
    let items: Vec<ListItem> = state
        .shares
        .iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .map(|share| {
            ListItem::from(share_row(share)).style(Style::default().add_modifier(Modifier::BOLD))
        })
        .collect();

    let list = List::new(items)
        .block(bordered_block(format!("Shares ({total})")))
        .highlight_style(Style::default().bg(state.palette.selection_bg()));
    let mut list_state = ListState::default();
    list_state.select(selected.map(|sel| sel.saturating_sub(start)));
    f.render_stateful_widget(list, list_area, &mut list_state);

    let chosen = selected.and_then(|sel| state.shares.get(sel));
    let lines = detail_lines(chosen, state);
    let para = Paragraph::new(lines.join("\n"))
        .block(bordered_block("Share details".to_string()))
        .wrap(Wrap { trim: false });
    f.render_widget(para, detail_area);
}
