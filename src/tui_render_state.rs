//! TUI rendering state abstraction for testing
//!
//! This module provides a render state that can be used by both the binary
//! and integration tests. The binary uses `AppState`, tests use this abstraction.

use crate::PeerRecord;
use crate::types::StringMap;
use std::collections::{BTreeMap, HashMap, VecDeque};

/// App-liveness flags for the renderer (Settings-tab node status + connection).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeStatus {
    /// Whether connected to the network.
    pub connected: bool,
    /// Whether the node is running (always true in the TUI).
    pub node_running: bool,
}

/// Minimum state needed to render a TUI frame
#[derive(Clone, Debug)]
pub struct TuiRenderState {
    /// Names of tabs for display (Chat, Contacts, Log, plus DM tabs)
    pub tab_titles: Vec<String>,
    /// Currently active tab index
    pub active_tab: usize,
    /// Broadcast messages in chat tab
    pub messages: VecDeque<String>,
    /// Peer ID of each message sender (None = sent by local user)
    pub message_peer_ids: VecDeque<Option<String>>,
    /// Message IDs for sent messages (None if from other peers)
    pub message_ids: VecDeque<Option<String>>,
    /// Receipt status for broadcast messages: `peer_id` -> (`msg_id` -> timestamp)
    pub broadcast_receipts: HashMap<String, HashMap<String, f64>>,
    /// Aggregated reaction chips by wire `msg_id` (US-008).
    pub reactions: HashMap<String, Vec<crate::reactions::ReactionSummary>>,
    /// Known peers (the raw device list backing `contacts`)
    pub peers: Vec<PeerRecord>,
    /// Canonical Contacts-tab rows (from `contacts::build_contact_rows`).
    /// The Contacts tab renders these; `peers` stays only as the device input.
    /// While a search is active this is the filtered view.
    pub contacts: Vec<crate::contacts::ContactRow>,
    /// Unfiltered canonical Contacts rows. Peer Info reads stats from here so
    /// hiding a row in the Contacts search never blanks its stats.
    pub contacts_all: Vec<crate::contacts::ContactRow>,
    /// Unfiltered Contacts count while a search query is applied, `None` when
    /// no filter is active. The renderer shows `Contacts (visible/total)` and a
    /// `No contacts match.` hint when this is `Some` (mirroring Flutter); using
    /// `Option` makes an active search without a known total unrepresentable.
    pub contact_search_total: Option<usize>,
    /// Contact IDs expanded into their device sub-rows on the Contacts tab.
    pub expanded_contact_ids: std::collections::HashSet<String>,
    /// Peers muted on this install (US-054), shown on the Peer Info tab.
    pub blocked_peers: std::collections::HashSet<String>,
    /// How many broadcasts *we* sent to each peer (bulk-loaded from
    /// `broadcast_recipients`), driving the peers-table "Broadcast" column.
    pub broadcast_sent_to_peer: HashMap<String, usize>,
    /// Peer IDs currently connected (subset of `peers`)
    pub connected_peer_ids: std::collections::HashSet<String>,
    /// Direct messages per peer
    pub dm_messages: BTreeMap<String, VecDeque<String>>,
    /// Message IDs for DMs per peer
    pub dm_message_ids: BTreeMap<String, VecDeque<Option<String>>>,
    /// DM receipt status per peer: (`source_peer`, timestamp)
    pub dm_receipts: HashMap<String, (String, f64)>,
    /// Peer IDs currently showing a live typing indicator (US-009).
    pub typing_peer_ids: std::collections::HashSet<String>,
    /// Current input text in message box
    pub input_text: String,
    /// Logs shown in the debug tab
    pub log_messages: VecDeque<String>,
    /// What the shared text input is currently capturing (chat/nickname/group).
    pub input_mode: crate::InputMode,
    /// Peer ID being edited (when `input_mode` is [`crate::InputMode::EditingNickname`])
    pub nickname_peer_id: String,
    /// App-liveness flags (connected + node running)
    pub runtime: RuntimeStatus,
    /// Number of connected peers
    pub peer_count: usize,
    /// Whether mouse is captured (for text input)
    pub mouse_capture: bool,
    /// Whether the terminal delivers kitty-keyboard encoding (selects the
    /// Peer-Info shortcut hint: Ctrl+I vs Ctrl+P on a Direct tab)
    pub kitty_keyboard_active: bool,
    /// Optional popup message to display
    pub popup: Option<String>,
    /// Canonical quoted header of the active conversation's reply draft.
    pub reply_preview: Option<String>,
    /// Explicitly selected transcript row of the active conversation, resolved
    /// against the current transcript. `None` means the verbs fall back to the
    /// newest row and no row is highlighted.
    pub message_selection_index: Option<usize>,
    /// Known groups with member counts (for the Groups list tab)
    pub group_summaries: Vec<crate::groups::GroupSummary>,
    /// Group messages per group: formatted display strings
    pub group_messages: BTreeMap<String, VecDeque<String>>,
    /// Message IDs for group messages per group
    pub group_message_ids: BTreeMap<String, VecDeque<Option<String>>>,
    /// Sender peer ID for each group message (None = sent by local user)
    pub group_message_peer_ids: BTreeMap<String, VecDeque<Option<String>>>,
    /// Scroll state per group chat: (offset, `auto_scroll`)
    pub group_scroll_state: BTreeMap<String, (usize, bool)>,
    /// Unread group messages per group while that chat is scrolled up
    pub group_unread_count: BTreeMap<String, usize>,
    /// Index of selected group in the Groups list tab
    pub group_selection: usize,
    /// Known file shares (owned and accepted) for the read-only Shares tab.
    pub shares: Vec<crate::shares::Share>,
    /// Highlighted share in the Shares list.
    pub share_selection: usize,
    /// Highlighted grant in the selected share's detail pane.
    pub share_grant_selection: usize,
    /// Grants by share id, used by the Shares tab detail pane.
    pub share_grants: HashMap<String, Vec<crate::shares::ShareGrant>>,
    /// Action label of the active grant prompt, when one is open.
    pub share_grant_action: Option<&'static str>,
    /// Selected access mode while adding a grant; `None` outside the prompt.
    pub share_grant_mode: Option<crate::shares::ShareMode>,
    /// Latest share-lane event (offer/update/rejection), shown on the Shares
    /// tab so swarm activity is visible without the Log tab.
    pub share_event_notice: Option<String>,
    /// Connected peers in canonical order, used by the Shares network pane.
    pub share_peer_ids: Vec<String>,
    /// Highlighted connected peer for Shares network actions.
    pub share_peer_selection: usize,
    /// Scroll offset for chat tab
    pub chat_scroll_offset: usize,
    /// Whether chat tab and log tab auto-scroll to bottom
    pub auto_scroll: crate::AutoScrollFlags,
    /// Number of broadcast messages that arrived while not at the bottom
    pub chat_unread_count: usize,
    /// Scroll offset for the log tab
    pub log_scroll_offset: usize,
    /// Scroll state per DM peer: (offset, `auto_scroll`)
    pub dm_scroll_state: BTreeMap<String, (usize, bool)>,
    /// Unread DMs per peer while that conversation is scrolled up
    pub dm_unread_count: BTreeMap<String, usize>,
    /// Scroll state for DM+broadcast view per peer
    pub dm_broadcast_scroll_state: BTreeMap<String, (usize, bool)>,
    /// Index of selected peer in peer list
    pub peer_selection: usize,
    /// Active sort column for the peer list (0=Name,1=DM,2=Broadcast,3=Last Seen,4=First Seen)
    pub peer_sort_column: usize,
    /// Whether the peer list is sorted ascending on the active column
    pub peer_sort_ascending: bool,
    /// First visible row of the peer table (scrolling viewport)
    pub peer_table_offset: usize,
    /// Map of peer ID -> local nickname set for that peer
    pub local_nicknames: HashMap<String, String>,
    /// Map of peer ID -> nickname received/announced by that peer
    pub received_nicknames: HashMap<String, String>,
    /// Current local nickname (for Settings tab)
    pub own_nickname: String,
    /// Local peer ID (for Settings tab)
    pub local_peer_id: String,
    /// Database URL (for Settings tab)
    pub db_url: String,
    /// Platform label, e.g. "Terminal (linux)" (for Settings tab)
    pub platform: String,
    /// Network size classification, e.g. "Small" (for Settings tab)
    pub network_size: String,
    /// Live network diagnostics snapshot (interfaces, reachability, DHT/relay).
    pub network: crate::net_diag::NetworkDetails,
    /// Persisted bootstrap/relay multiaddrs (Settings tab).
    pub network_peers: Vec<crate::network_config::NetworkPeer>,
    /// Canonical palette used by every renderer (US-025/US-026). Snapshotted
    /// from Rust settings so the TUI and Flutter share one colour definition.
    pub palette: crate::theme::Palette,
    /// Highlighted row in the Settings network-peers list.
    pub network_peer_selection: usize,
    /// Kind captured by the active add prompt (`None` = not adding), used by
    /// the input-section title.
    pub network_peer_kind: Option<String>,
    /// Epoch seconds of last peer disconnect, if any (for Settings tab)
    pub last_connection_lost: Option<f64>,
    /// Peer ID we last disconnected from, if any (for Settings tab)
    pub last_connection_peer: Option<String>,
    /// This install's user ID (None before provisioning) — Settings tab
    pub own_user_id: Option<String>,
    /// Devices bound into this install's user: `(peer_id, bound_at)` —
    /// Settings tab
    pub own_user_devices: Vec<(String, String)>,
}

impl Default for TuiRenderState {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiRenderState {
    /// Creates a new empty [`TuiRenderState`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            tab_titles: vec![
                "Chat".into(),
                "Contacts".into(),
                "Groups".into(),
                "Log".into(),
                "Settings".into(),
                "Shares".into(),
            ],
            active_tab: 0,
            messages: VecDeque::new(),
            message_peer_ids: VecDeque::new(),
            message_ids: VecDeque::new(),
            broadcast_receipts: HashMap::new(),
            reactions: HashMap::new(),
            peers: Vec::new(),
            contacts: Vec::new(),
            contacts_all: Vec::new(),
            contact_search_total: None,
            expanded_contact_ids: std::collections::HashSet::new(),
            blocked_peers: std::collections::HashSet::new(),
            broadcast_sent_to_peer: HashMap::new(),
            connected_peer_ids: std::collections::HashSet::new(),
            dm_messages: BTreeMap::new(),
            dm_message_ids: BTreeMap::new(),
            dm_receipts: HashMap::new(),
            typing_peer_ids: std::collections::HashSet::new(),
            group_summaries: Vec::new(),
            group_messages: BTreeMap::new(),
            group_message_ids: BTreeMap::new(),
            group_message_peer_ids: BTreeMap::new(),
            group_scroll_state: BTreeMap::new(),
            group_unread_count: BTreeMap::new(),
            group_selection: 0,
            shares: Vec::new(),
            share_grants: HashMap::new(),
            share_selection: 0,
            share_grant_selection: 0,
            share_grant_action: None,
            share_grant_mode: None,
            share_event_notice: None,
            share_peer_ids: Vec::new(),
            share_peer_selection: 0,
            input_text: String::new(),
            log_messages: VecDeque::new(),
            input_mode: crate::InputMode::Chat,
            nickname_peer_id: String::new(),
            runtime: RuntimeStatus {
                connected: true,
                node_running: true,
            },
            peer_count: 0,
            mouse_capture: false,
            kitty_keyboard_active: false,
            popup: None,
            reply_preview: None,
            message_selection_index: None,
            chat_scroll_offset: 0,
            auto_scroll: crate::AutoScrollFlags {
                chat: true,
                log: true,
            },
            chat_unread_count: 0,
            log_scroll_offset: 0,
            dm_scroll_state: BTreeMap::new(),
            dm_unread_count: BTreeMap::new(),
            dm_broadcast_scroll_state: BTreeMap::new(),
            peer_selection: 0,
            peer_sort_column: 3,
            peer_sort_ascending: false,
            peer_table_offset: 0,
            own_nickname: String::new(),
            local_peer_id: String::new(),
            db_url: String::new(),
            platform: String::new(),
            network_size: String::new(),
            network: crate::net_diag::NetworkDetails::default(),
            network_peers: Vec::new(),
            palette: crate::theme::Palette::default(),
            network_peer_selection: 0,
            network_peer_kind: None,
            last_connection_lost: None,
            last_connection_peer: None,
            own_user_id: None,
            own_user_devices: Vec::new(),
            local_nicknames: HashMap::new(),
            received_nicknames: HashMap::new(),
        }
    }

    /// True while a modal prompt owns the shared input. Mirrors
    /// [`crate::InputMode::is_modal_editor`] so both renderers and input gates
    /// agree on which modes keep the keyboard.
    #[must_use]
    pub const fn is_modal_editor(&self) -> bool {
        self.input_mode.is_modal_editor()
    }

    /// True while the shared text input captures a nickname edit (own, per-peer
    /// announced, or a peer's local display name).
    #[must_use]
    pub const fn is_editing_nickname(&self) -> bool {
        matches!(
            self.input_mode,
            crate::InputMode::EditingNickname | crate::InputMode::EditingLocalNickname
        )
    }

    /// True while the shared input captures a local edit of a sent message.
    #[must_use]
    pub const fn is_editing_message(&self) -> bool {
        matches!(self.input_mode, crate::InputMode::EditingMessage)
    }

    /// True while the shared input captures a reaction emoji (US-008).
    #[must_use]
    pub const fn is_adding_reaction(&self) -> bool {
        matches!(self.input_mode, crate::InputMode::AddingReaction)
    }

    /// True while the shared input captures a private-share grant user id.
    #[must_use]
    pub const fn is_editing_share_grant(&self) -> bool {
        matches!(self.input_mode, crate::InputMode::EditingShareGrant)
    }

    /// True while the editor sets the local display name of `nickname_peer_id`.
    #[must_use]
    pub const fn is_editing_local_nickname(&self) -> bool {
        matches!(self.input_mode, crate::InputMode::EditingLocalNickname)
    }

    /// True while the shared text input captures a group name.
    #[must_use]
    pub const fn is_creating_group(&self) -> bool {
        matches!(self.input_mode, crate::InputMode::CreatingGroup)
    }
    /// True while the shared input captures a peer ID / multiaddr to dial.
    #[must_use]
    pub const fn is_dialing_peer(&self) -> bool {
        matches!(self.input_mode, crate::InputMode::DialingPeer)
    }

    /// True while the shared input captures a bootstrap/relay multiaddr.
    #[must_use]
    pub const fn is_adding_network_peer(&self) -> bool {
        matches!(self.input_mode, crate::InputMode::AddingNetworkPeer)
    }

    /// True while the shared text input captures a Contacts search query.
    #[must_use]
    pub const fn is_searching_contacts(&self) -> bool {
        matches!(self.input_mode, crate::InputMode::SearchingContacts)
    }

    /// Create with sample data for testing
    #[must_use]
    pub fn with_sample_data() -> Self {
        let mut messages = VecDeque::new();
        messages.push_back("[You] Hello world".into());
        messages.push_back("[Peer1] How are you?".into());
        messages.push_back("[You] I'm good!".into());

        let peers = vec![
            PeerRecord {
                peer_id: "12D3KooWH123456".into(),
                first_seen: "Alice".into(),
                last_seen: "Online".into(),
            },
            PeerRecord {
                peer_id: "12D3KooWH789012".into(),
                first_seen: "Bob".into(),
                last_seen: "Online".into(),
            },
        ];

        let mut dm_messages = BTreeMap::new();
        dm_messages.insert("Alice".into(), VecDeque::new());
        let mut dm_message_ids = BTreeMap::new();
        dm_message_ids.insert("Alice".into(), VecDeque::new());

        let mut state = Self {
            tab_titles: vec![
                "Chat".into(),
                "Contacts".into(),
                "Log".into(),
                "DM: Alice".into(),
            ],
            messages,
            peers,
            dm_messages,
            dm_message_ids,
            peer_count: 2,
            ..Self::new()
        };
        state.contacts = contacts_for_peers(&state.peers);
        state.contacts_all = state.contacts.clone();
        state
    }

    /// Add a message to chat (`peer_id` of `None` means local user)
    pub fn add_message(&mut self, msg: impl Into<String>) {
        self.messages.push_back(msg.into());
        self.message_peer_ids.push_back(None);
    }

    /// Add a peer
    pub fn add_peer(
        &mut self,
        peer_id: impl Into<String>,
        first_seen: impl Into<String>,
        last_seen: impl Into<String>,
    ) {
        self.peers.push(PeerRecord {
            peer_id: peer_id.into(),
            first_seen: first_seen.into(),
            last_seen: last_seen.into(),
        });
        self.contacts = contacts_for_peers(&self.peers);
        self.contacts_all = self.contacts.clone();
    }

    /// Add a DM message for a peer
    pub fn add_dm_message(&mut self, peer: impl Into<String>, msg: impl Into<String>) {
        let peer = peer.into();
        let entry = self.dm_messages.entry(peer).or_default();
        entry.push_back(msg.into());
    }
}

/// Collapse a device list into single-device contact rows via the shared
/// grouping builder (no identity DB: tests have no bindings, so every device
/// stays an unbound single row — exactly the pre-contacts rendering).
#[must_use]
fn contacts_for_peers(peers: &[PeerRecord]) -> Vec<crate::contacts::ContactRow> {
    let devices: Vec<crate::contacts::ContactDeviceInput> = peers
        .iter()
        .map(|p| crate::contacts::ContactDeviceInput {
            peer_id: p.peer_id.clone(),
            first_seen: p.first_seen.clone(),
            last_seen: p.last_seen.clone(),
            received_nickname: None,
            local_nickname: None,
        })
        .collect();
    let device_display_names: HashMap<String, String> = peers
        .iter()
        .map(|p| {
            (
                p.peer_id.clone(),
                crate::get_peer_display_name_or_short(&p.peer_id),
            )
        })
        .collect();
    let stats: HashMap<String, crate::messages::PeerMessageStats> = HashMap::new();
    crate::contacts::build_contact_rows(
        &devices,
        &device_display_names,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &stats,
    )
}

/// Calculate visible items and effective offset for string-based messages with auto/manual scroll
#[must_use]
pub fn calc_visible_strings(
    messages: &VecDeque<String>,
    auto_scroll: bool,
    scroll_offset: usize,
    text_width: usize,
    usable_height: usize,
) -> (usize, usize) {
    let msgs: Vec<String> = messages.iter().cloned().collect();
    calc_visible_impl(&msgs, auto_scroll, scroll_offset, usable_height, |m| {
        count_lines(m, text_width)
    })
}

/// Count wrapped lines of text, accounting for ANSI codes and terminal width.
///
/// Uses display width (CJK/wide characters count as two columns) rather than
/// byte length, so wide-character messages wrap like the terminal does.
#[must_use]
pub fn count_lines(text: &str, text_width: usize) -> usize {
    use unicode_width::UnicodeWidthStr;

    if text_width == 0 || text.is_empty() {
        return 1;
    }
    let clean_text = crate::logging::strip_ansi_codes(text);
    let lines: Vec<&str> = clean_text.split('\n').collect();

    let mut total: usize = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            if i < lines.len().saturating_sub(1) {
                total = total.saturating_add(1);
            }
        } else {
            total = total.saturating_add(line.width().div_ceil(text_width));
        }
    }
    total.max(1)
}

const MIN_VISIBLE: usize = 1;

/// Number of terminal rows a `ListItem` occupies in a ratatui `List`.
///
/// `List` renders each newline-separated segment as one row and clips lines
/// that are longer than the pane (it never width-wraps), so the row count is
/// the number of newline-delimited segments, not `count_lines`' wrapped
/// estimate. Chat, DM, broadcast, and log panes all render their messages via
/// `List`, so the scroll math, the renderer, and the click hit-testing must
/// all agree on this count.
#[must_use]
pub fn list_item_lines(text: &str) -> usize {
    text.split('\n').count()
}

/// Maximum top-item index for a `List`-rendered pane.
///
/// Scrolling is bottom-aligned: the newest item can sit at most level with the
/// bottom of the pane, so the offset never exceeds `len - visible`, where
/// `visible` is how many leading items fit. A larger offset would only reveal
/// empty space below the newest message and would stop auto-scroll from
/// re-engaging when the user scrolls back down to the bottom.
#[must_use]
pub fn max_list_scroll_offset(line_counts: &[usize], usable_height: usize) -> usize {
    if line_counts.is_empty() {
        return 0;
    }
    let mut used: usize = 0;
    let mut visible: usize = 0;
    for &lines in line_counts {
        if used > 0 && used.saturating_add(lines) > usable_height {
            break;
        }
        used = used.saturating_add(lines);
        visible = visible.saturating_add(1);
    }
    line_counts.len().saturating_sub(visible.max(MIN_VISIBLE))
}

/// Compute the visible window for `List`-rendered panes (chat, DM, log).
///
/// Same `(visible, effective_offset)` contract as [`calc_visible_strings`],
/// but rows-per-item uses [`list_item_lines`] so it matches how ratatui's
/// `List` actually lays out items (no width wrapping).
#[must_use]
pub fn calc_visible_list_items(
    messages: &VecDeque<String>,
    auto_scroll: bool,
    scroll_offset: usize,
    usable_height: usize,
) -> (usize, usize) {
    let msgs: Vec<String> = messages.iter().cloned().collect();
    calc_visible_impl(
        &msgs,
        auto_scroll,
        scroll_offset,
        usable_height,
        list_item_lines,
    )
}

/// Translate a transcript's global selected row into the relative index
/// ratatui's `ListState` expects. `None` when no row is selected or the chosen
/// row is scrolled out of the visible window.
#[must_use]
pub const fn selected_visible_index(
    selected: Option<usize>,
    effective_offset: usize,
    visible: usize,
) -> Option<usize> {
    match selected {
        Some(index) if index >= effective_offset => {
            let relative = index.saturating_sub(effective_offset);
            if relative < visible {
                Some(relative)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn calc_visible_impl<F>(
    messages: &[String],
    auto_scroll: bool,
    scroll_offset: usize,
    usable_height: usize,
    get_lines: F,
) -> (usize, usize)
where
    F: Fn(&str) -> usize,
{
    let line_counts: Vec<usize> = messages.iter().map(|m| get_lines(m)).collect();
    calc_visible_counts(&line_counts, auto_scroll, scroll_offset, usable_height)
}

/// Shared `(visible, effective_offset)` window math over per-item line counts.
///
/// Both the string-based and `List`-based renderers funnel through this, so a
/// revealed cursor and the frame that highlights it cannot disagree on which
/// rows are on screen.
fn calc_visible_counts(
    line_counts: &[usize],
    auto_scroll: bool,
    scroll_offset: usize,
    usable_height: usize,
) -> (usize, usize) {
    let total = line_counts.len();
    if total == 0 {
        return (0, 0);
    }
    let mut used = 0usize;
    let mut visible = 0usize;
    if auto_scroll {
        for &lines in line_counts.iter().rev() {
            if used > 0 && used.saturating_add(lines) > usable_height {
                break;
            }
            used = used.saturating_add(lines);
            visible = visible.saturating_add(1);
        }
        (visible, total.saturating_sub(visible))
    } else {
        for &lines in line_counts.iter().skip(scroll_offset) {
            if used > 0 && used.saturating_add(lines) > usable_height {
                break;
            }
            used = used.saturating_add(lines);
            visible = visible.saturating_add(1);
        }
        (visible.max(MIN_VISIBLE), scroll_offset)
    }
}

/// New `(scroll_offset, auto_scroll)` that brings `target` into view.
///
/// Bare arrow scrolling never calls this: it is only used after the keyboard
/// cursor moves outside the visible window. Moving up pins the target at the
/// top; moving down walks back from it to find the smallest window that still
/// fits, re-engaging auto-scroll when that window is the newest one.
#[must_use]
pub fn reveal_list_item_offset(
    line_counts: &[usize],
    usable_height: usize,
    current_offset: usize,
    auto_scroll: bool,
    target: usize,
) -> (usize, bool) {
    let total = line_counts.len();
    if total == 0 {
        return (current_offset, auto_scroll);
    }
    let target = target.min(total.saturating_sub(1));
    let (visible, effective_offset) =
        calc_visible_counts(line_counts, auto_scroll, current_offset, usable_height);
    if target >= effective_offset && target < effective_offset.saturating_add(visible) {
        return (current_offset, auto_scroll);
    }
    if target < effective_offset {
        return (target, false);
    }
    let mut used = 0usize;
    let mut start = target;
    // Walk backwards over the target's *prefix*, not the transcript tail:
    // `distance` is how far an earlier item sits from the target. Iterating the
    // tail would sum unrelated newer rows and could leave the target outside
    // the window when messages have uneven heights.
    for (distance, lines) in line_counts
        .iter()
        .take(target.saturating_add(1))
        .rev()
        .copied()
        .enumerate()
    {
        if used > 0 && used.saturating_add(lines) > usable_height {
            break;
        }
        used = used.saturating_add(lines);
        start = target.saturating_sub(distance);
    }
    // Compare against the renderer's actual bottom window, not
    // `max_list_scroll_offset`: with uneven message heights the largest legal
    // offset can show fewer rows than the auto-scrolled tail. Reaching the
    // tail must re-engage auto-scroll so the next message still follows.
    let (_, bottom_offset) = calc_visible_counts(line_counts, true, 0, usable_height);
    if start >= bottom_offset {
        (bottom_offset, true)
    } else {
        (start, false)
    }
}

/// Return a checkmark prefix (`"v "`) if any peer has acknowledged the given
/// broadcast message, or two spaces otherwise.
#[must_use]
pub fn broadcast_receipt_prefix(
    msg_id: Option<&str>,
    broadcast_receipts: &StringMap<StringMap<f64>>,
) -> &'static str {
    msg_id.map_or("  ", |msg_id| {
        let confirmed = broadcast_receipts
            .get(msg_id)
            .map_or(0, std::collections::HashMap::len);
        if confirmed == 0 { "  " } else { "v " }
    })
}

/// Return a checkmark prefix (`"v "`) if the recipient has acknowledged the
/// given direct message, or two spaces otherwise.
#[must_use]
pub fn dm_receipt_prefix(
    msg_id: Option<&str>,
    dm_receipts: &StringMap<(String, f64)>,
) -> &'static str {
    match msg_id {
        Some(msg_id) if dm_receipts.contains_key(msg_id) => "v ",
        _ => "  ",
    }
}

/// Render the aggregated reactions on one message as a compact inline suffix,
/// e.g. `"  [👍 2] [❤️ 1*]"`.
///
/// A trailing `*` marks a chip this install reacted with. Returns an empty
/// string when the message has no reactions, so callers can append it verbatim
/// to a rendered message line (US-008).
#[must_use]
pub fn reaction_suffix<S: std::hash::BuildHasher>(
    msg_id: Option<&str>,
    reactions: &HashMap<String, Vec<crate::reactions::ReactionSummary>, S>,
) -> String {
    let Some(summaries) = msg_id.and_then(|id| reactions.get(id)) else {
        return String::new();
    };
    let mut out = String::new();
    for summary in summaries {
        out.push_str(" [");
        out.push_str(&summary.emoji);
        out.push(' ');
        out.push_str(&summary.count.to_string());
        if summary.mine {
            out.push('*');
        }
        out.push(']');
    }
    out
}

/// Maps a clicked terminal row to the message index it falls within.
///
/// Uses each message's rendered line count and the starting content row to
/// locate the message. Returns `None` if the click was above the content area
/// or past the last message.
#[must_use]
pub fn row_to_visible_index(
    line_counts: &[usize],
    first_content_row: usize,
    click_row: usize,
) -> Option<usize> {
    if click_row < first_content_row {
        return None;
    }
    let mut current_row = first_content_row;

    for (idx, line_count) in line_counts.iter().copied().enumerate() {
        let message_end_row = current_row.saturating_add(line_count);
        if click_row < message_end_row {
            return Some(idx);
        }
        current_row = message_end_row;
    }

    None
}

/// Get the tab content based on active tab index
#[must_use]
pub fn get_tab_content(state: &TuiRenderState) -> crate::tui_tabs::TabContent {
    let tab_title = state
        .tab_titles
        .get(state.active_tab)
        .cloned()
        .unwrap_or_default();

    if tab_title.starts_with("DM: ") {
        let peer = tab_title.trim_start_matches("DM: ").to_string();
        crate::tui_tabs::TabContent::Direct(peer)
    } else if tab_title.starts_with("Group: ") {
        let group_id = tab_title.trim_start_matches("Group: ").to_string();
        crate::tui_tabs::TabContent::GroupChat(group_id)
    } else if tab_title.starts_with("Info: ") {
        let peer = tab_title.trim_start_matches("Info: ").to_string();
        crate::tui_tabs::TabContent::PeerInfo(peer)
    } else if tab_title == "Contacts" {
        crate::tui_tabs::TabContent::Contacts
    } else if tab_title == "Groups" {
        crate::tui_tabs::TabContent::Groups
    } else if tab_title == "Log" {
        crate::tui_tabs::TabContent::Log
    } else if tab_title == "Settings" {
        crate::tui_tabs::TabContent::Settings
    } else if tab_title == "Shares" {
        crate::tui_tabs::TabContent::Shares
    } else {
        crate::tui_tabs::TabContent::Chat
    }
}

#[cfg(test)]
#[path = "../tests/unit/unit_tui_render_state.rs"]
mod tests;
