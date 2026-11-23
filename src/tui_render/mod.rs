//! TUI rendering functions for both binary and tests.

mod chat;
mod groups;
mod log;
mod peer_info;
mod peers;
mod settings;

pub use chat::{render_chat_content, render_dm_content};
pub use groups::{render_group_chat_content, render_groups_content};
pub use log::render_log_content;
pub use peer_info::render_peer_info_content;
pub use peers::render_peers_content;
pub use settings::render_settings_content;

use crate::fmt::short_peer_id;
use crate::tui_render_state::{TuiRenderState, get_tab_content};
use crate::tui_tabs::TabContent;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
};

/// Render a full TUI frame
pub fn render_frame(f: &mut ratatui::Frame, state: &mut TuiRenderState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    let default_rect = Rect::default();
    let tab_area = chunks.first().unwrap_or(&default_rect);
    let content_area = chunks.get(1).unwrap_or(&default_rect);
    let input_area = chunks.get(2).unwrap_or(&default_rect);
    let shortcut_area = chunks.get(3).unwrap_or(&default_rect);
    let status_area = chunks.get(4).unwrap_or(&default_rect);

    render_tabs(f, *tab_area, state);

    let tab_content = get_tab_content(state);
    render_tab_content(f, *content_area, &tab_content, state);

    render_input_section(f, *input_area, state, &tab_content);
    render_shortcuts(f, *shortcut_area, &tab_content, state.kitty_keyboard_active);
    render_status_bar(f, *status_area, state);

    if let Some(ref text) = state.popup {
        if text.as_str() == shortcuts_help_text() {
            render_shortcuts_help_popup(f, text.clone(), state.palette);
        } else {
            render_popup(f, text.clone(), state.palette);
        }
    }
}

/// Render the tab bar
pub fn render_tabs(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let titles: Vec<&str> = state
        .tab_titles
        .iter()
        .map(std::string::String::as_str)
        .collect();
    let tabs = Tabs::new(titles)
        .style(Style::default().fg(state.palette.accent()))
        .select(state.active_tab)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_widget(tabs, area);
}

/// Render tab content
pub fn render_tab_content(
    f: &mut ratatui::Frame,
    area: Rect,
    tab_content: &TabContent,
    state: &mut TuiRenderState,
) {
    match tab_content {
        TabContent::Chat => render_chat_content(f, area, state),
        TabContent::Contacts => render_peers_content(f, area, state),
        TabContent::Groups => render_groups_content(f, area, state),
        TabContent::GroupChat(group_id) => render_group_chat_content(f, area, group_id, state),
        TabContent::Direct(peer_id) => render_dm_content(f, area, peer_id, state),
        TabContent::Log => render_log_content(f, area, state),
        TabContent::Settings => render_settings_content(f, area, state),
        TabContent::PeerInfo(peer_id) => render_peer_info_content(f, area, peer_id, state),
    }
}

/// Shared bordered list/paragraph block with the given title.
pub(crate) fn bordered_block(title: String) -> Block<'static> {
    Block::default().title(title).borders(Borders::ALL)
}

/// Render input section
pub fn render_input_section(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &TuiRenderState,
    tab_content: &TabContent,
) {
    let title = if state.is_adding_reaction() {
        "Reaction emoji".to_string()
    } else if state.is_editing_message() {
        "Edit message (local only)".to_string()
    } else if state.is_dialing_peer() {
        "Dial peer address".to_string()
    } else if state.is_adding_network_peer() {
        format!(
            "Add {} multiaddr",
            state.network_peer_kind.as_deref().unwrap_or("network peer")
        )
    } else if state.is_searching_contacts() {
        "Search Contacts".to_string()
    } else if state.is_editing_nickname() {
        let label = if state.is_editing_local_nickname() {
            "Local Nickname"
        } else {
            "Edit Nickname"
        };
        format!("{label} ({})", short_peer_id(&state.nickname_peer_id))
    } else if let Some(preview) = state.reply_preview.as_deref() {
        format!("Reply: {preview}")
    } else {
        "Input".to_string()
    };

    let input_block = Block::default().title(title).borders(Borders::ALL);

    if tab_content.is_input_enabled()
        || state.is_editing_nickname()
        || state.is_editing_message()
        || state.is_adding_reaction()
        || state.is_searching_contacts()
        || state.is_dialing_peer()
        || state.is_adding_network_peer()
    {
        let inner_area = input_block.inner(area);
        f.render_widget(input_block, area);
        let input = Paragraph::new(state.input_text.as_str());
        f.render_widget(input, inner_area);
    } else {
        f.render_widget(input_block, area);
    }
}

/// Shortcut hint line for the given tab.
///
/// Bindings are documented only where they are active: Ctrl+I (kitty) and
/// Ctrl+P (universal) on a Direct/DM tab, `i` on Contacts, `n` (local
/// nickname) on a Peer Info tab, `Del` on the three chat tabs.
#[must_use]
pub const fn shortcuts_text(tab_content: &TabContent, kitty: bool) -> &'static str {
    match tab_content {
        TabContent::Contacts => {
            "Tab: next | Up/Down: select | Enter: open DM | i: Peer Info | c: call | d: link device | /: search | F12: mouse | ?: help | Ctrl+Q: quit"
        }
        TabContent::PeerInfo(_) => {
            "Tab: next | Enter: open DM | c: call | d: link device | b: block/unblock | n: set local nickname | F12: mouse | ?: help | Ctrl+Q: quit"
        }
        TabContent::Groups => {
            "Tab: next | Up/Down: select | Enter: open chat | g: create/join | d: leave | F12: mouse | ?: help | Ctrl+Q: quit"
        }
        TabContent::Settings => {
            "Tab: next | n: nickname | d: dial | b/r: add bootstrap/relay | x: remove | l: LAN | t: theme | c: accent | s: sound | o: desktop alert | F12: mouse | ?: help | Ctrl+Q: quit"
        }
        TabContent::Direct(_) if kitty => {
            "Tab: next | Ctrl+I/Ctrl+P: Peer Info | Ctrl+R: reply | Ctrl+Y: react | Ctrl+C: copy | Ctrl+O: open file | PgUp/PgDn: scroll | Home/End: jump | Enter: send | Del: delete sent | Ctrl+E: edit sent | F12: mouse | ?: help | Ctrl+Q: quit"
        }
        TabContent::Direct(_) => {
            "Tab: next | Ctrl+P: Peer Info | Ctrl+R: reply | Ctrl+Y: react | Ctrl+C: copy | Ctrl+O: open file | PgUp/PgDn: scroll | Home/End: jump | Enter: send | Del: delete sent | Ctrl+E: edit sent | F12: mouse | ?: help | Ctrl+Q: quit"
        }
        TabContent::Chat | TabContent::GroupChat(_) => {
            "Tab: next | Ctrl+R: reply | Ctrl+Y: react | Ctrl+C: copy | Ctrl+O: open file | PgUp/PgDn: scroll | Home/End: jump | Enter: send | Del: delete sent | Ctrl+E: edit sent | F12: mouse | ?: help | Ctrl+Q: quit"
        }
        TabContent::Log => {
            "Tab: next | PgUp/PgDn: scroll | Home/End: jump | Ctrl+C: copy all | c: clear logs | F12: mouse | ?: help | Ctrl+Q: quit"
        }
    }
}

/// Compact modal table shown when the TUI receives `?`.
///
/// Kept here (rather than in the binary) so the exact payload is testable and
/// the popup can be recognized when rendering. It deliberately names only
/// bindings that are live somewhere in the TUI; the full per-tab reference is
/// `KEYBOARD_SHORTCUTS.md`.
#[must_use]
pub const fn shortcuts_help_text() -> &'static str {
    r"Tab / Shift+Tab  Next / previous tab      Ctrl+W  Close transient tab
?  This help                             F12  Mouse   Ctrl+Q  Quit
Up/Down  Move   PgUp/PgDn  Page   Home/End  First / last
Enter  Send / open / save   Shift+Enter  Newline   Esc  Cancel / back

Chat / DM / Group: click a message -> sender's Peer Info
Chat / DM / Group: Ctrl+R -> reply to newest message (Esc clears)
Chat / DM / Group: Ctrl+Y -> react to newest message with an emoji
Chat / DM / Group: Ctrl+C -> copy newest message (OSC 52)
Chat / DM / Group: Ctrl+O -> open newest downloaded attachment
Chat / DM / Group: Del -> delete newest sent message (local only)
Chat / DM / Group: Ctrl+E -> edit newest sent message (local only)
Chat / DM / Group: /file <path> -> attach and send a local file
DM: Ctrl+I (kitty) / Ctrl+P -> partner's Peer Info

Contacts: i Info | e expand | c call | d link device | / search | 1-5 sort | o reverse
Groups: g create/join | d leave | y confirm leave
Peer Info: n local nickname | c call | d link device | b block / unblock
Log: Ctrl+C copy all logs | c clear logs
Settings: n own nickname | d dial | b/r add peer | x remove | l LAN only | t dark/light | c accent | s sound | o desktop alert

Prompts own the input; Esc cancels. Full reference: KEYBOARD_SHORTCUTS.md"
}

/// Render the dedicated, larger popup used by [`shortcuts_help_text`].
pub fn render_shortcuts_help_popup(
    f: &mut ratatui::Frame,
    text: String,
    palette: crate::theme::Palette,
) {
    render_bordered_popup(
        f,
        text,
        90,
        80,
        "Keyboard shortcuts — any key to close",
        palette,
    );
}

/// Render shortcuts help, tailored to the active tab
pub fn render_shortcuts(f: &mut ratatui::Frame, area: Rect, tab_content: &TabContent, kitty: bool) {
    let shortcuts = Paragraph::new(shortcuts_text(tab_content, kitty));
    f.render_widget(shortcuts, area);
}

/// Render status bar
pub fn render_status_bar(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let mouse = if state.mouse_capture { "ON" } else { "OFF" };
    let conn = if state.runtime.connected {
        "Connected"
    } else {
        "Disconnected"
    };
    let status = Paragraph::new(format!("{conn} [Mouse: {mouse}]"));
    f.render_widget(status, area);
}

/// Render an ordinary informational popup.
pub fn render_popup(f: &mut ratatui::Frame, text: String, palette: crate::theme::Palette) {
    render_bordered_popup(f, text, 70, 40, "Details", palette);
}

/// Shared centered popup geometry for notices and the shortcut-help overlay.
fn render_bordered_popup(
    f: &mut ratatui::Frame,
    text: String,
    width_percent: u16,
    height_percent: u16,
    title: &'static str,
    palette: crate::theme::Palette,
) {
    let area = f.area();
    let w = area.width.saturating_mul(width_percent) / 100;
    let h = area.height.saturating_mul(height_percent) / 100;
    let popup = Rect {
        x: area.x.saturating_add(area.width.saturating_sub(w) / 2),
        y: area.y.saturating_add(area.height.saturating_sub(h) / 2),
        width: w.max(20).min(area.width),
        height: h.max(6).min(area.height),
    };

    f.render_widget(Clear, popup);
    let p = Paragraph::new(text)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(Style::default().bg(palette.popup_bg())),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    f.render_widget(p, popup);
}
