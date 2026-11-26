//! Read-only Shares tab: known shares on the left, the highlighted share's
//! manifest on the right. Mirrors Flutter's list → detail flow without any
//! mutable actions; creation/binding/snapshots stay native operations.

use crate::shares::{Share, ShareKind};
use crate::tui_helpers::peer_table_visible_range;
use crate::tui_render::bordered_block;
use crate::tui_render_state::TuiRenderState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
};

/// Wire value as the user-facing label (`serde` has no `Display` impl).
const fn kind_label(kind: ShareKind) -> &'static str {
    match kind {
        ShareKind::Private => "private",
        ShareKind::Public => "public",
    }
}

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
fn detail_lines(share: Option<&Share>, notice: Option<&str>) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(notice) = notice {
        lines.push(format!("Last event: {notice}"));
        lines.push(String::new());
    }
    let Some(share) = share else {
        lines.push("No shares known to this install.".to_string());
        return lines;
    };
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
    match share.manifest.as_ref() {
        Some(manifest) => {
            lines.push(format!("Files ({}):", manifest.entries.len()));
            for entry in &manifest.entries {
                lines.push(format!(
                    "  {} ({})",
                    entry.path,
                    crate::files::format_file_size(entry.size)
                ));
            }
        }
        None => lines.push("Files: no manifest yet".to_string()),
    }
    lines
}

/// Render the Shares tab: a selectable share list and the selected share's
/// metadata, grants summary and manifest file list.
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
    let lines = detail_lines(chosen, state.share_event_notice.as_deref());
    let para = Paragraph::new(lines.join("\n"))
        .block(bordered_block("Share details".to_string()))
        .wrap(Wrap { trim: false });
    f.render_widget(para, detail_area);
}
