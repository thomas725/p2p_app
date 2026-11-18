use super::{DynamicTabs, TextArea};
use p2p_app::contacts::ContactDeviceInput;
use p2p_app::messages::PeerMessageStats;
use p2p_app::tui_helpers::PeerMessageMap;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// Channel capacity for inter-task communication (events per task)
pub const CHANNEL_CAPACITY: usize = 100;
/// Maximum direct messages to keep per peer conversation.
pub use p2p_app::types::MAX_DM_HISTORY;
/// Maximum messages to keep in memory (older messages are dropped).
pub use p2p_app::types::MAX_MESSAGE_HISTORY;
/// Input poll interval in milliseconds
pub const FRAME_TIME_MS: u64 = 16;

/// Trim a `VecDeque` to a maximum length, removing oldest (front) items.
pub fn trim_history<T>(queue: &mut VecDeque<T>, limit: usize) {
    while queue.len() > limit {
        queue.pop_front();
    }
}

pub type SharedState = Arc<tokio::sync::Mutex<AppState>>;

/// Shared application state for all tasks
///
/// This struct centralizes all mutable state needed by the TUI.
/// Only the `CommandProcessor` task directly mutates this state.
/// Other tasks:
/// - **`RenderLoop`**: Read-only access to render current state
/// - **`InputHandler`**: No direct access, sends `InputEvent` to `CommandProcessor`
/// - **`SwarmHandler`**: No direct access, sends `SwarmEvent` to `CommandProcessor`
///
/// Terminal input/output environment flags for the TUI runtime.
#[derive(Clone, Copy, Debug)]
pub struct TerminalEnv {
    /// Whether the terminal genuinely delivers kitty-keyboard encoding (from
    /// crossterm's `supports_keyboard_enhancement()`). On `WezTerm` (kitty active)
    /// `Ctrl+I` arrives as `Char('i')`+`CONTROL` and opens `PeerInfo` on a Direct tab.
    /// On collapsed terminals (Konsole) Ctrl+I (and Ctrl+Shift+I) collapse to
    /// the bare `0x09` Tab byte, so `Ctrl+I` opens `PeerInfo` only on kitty
    /// terminals. Ctrl+P (`Char('p')`+CONTROL from 0x10 DLE) works on every
    /// terminal as a universal fallback. Bare `Tab` always cycles tabs either way.
    pub kitty_keyboard_active: bool,
    /// Whether curses-style mouse events are captured (F12 toggles it).
    pub mouse_capture: bool,
    /// One-shot flag: display names have been resolved for every known peer the
    /// first time the Contacts tab is drawn (warms the library's name cache).
    pub peer_names_warmed: bool,
}

/// On-disk data loaded before the UI starts (messages, peers, receipts, groups).
///
/// [`AppState::new`] takes these as one struct so construction stays under the
/// `too_many_arguments` threshold; callers that load nothing real (tests) use
/// [`InitialStateData::default`].
#[derive(Default)]
pub struct InitialStateData {
    /// Preloaded broadcast-message history (display-ready + aligned IDs).
    pub messages: VecDeque<p2p_app::DisplayMessage>,
    /// Message IDs aligned with `messages`.
    pub message_ids: VecDeque<Option<String>>,
    /// DB row IDs aligned with `messages` (local-only deletion, US-005).
    /// `Some(id)` only for rows we sent; `None` for incoming/live rows.
    pub message_db_ids: VecDeque<Option<i32>>,
    /// Known peers, sorted most-recently-seen first.
    pub peers: VecDeque<p2p_app::PeerRecord>,
    /// Broadcast receipts: `msg_id` -> (`peer_id` -> `received_at`).
    pub broadcast_receipts: HashMap<String, HashMap<String, f64>>,
    /// DM receipts: `msg_id` -> (`source_peer`, `received_at`).
    pub dm_receipts: HashMap<String, (String, f64)>,
    /// Known groups with member counts.
    pub groups: Vec<p2p_app::groups::GroupSummary>,
}

/// Single-writer UI state (see the module doc above).
///
/// This single-writer pattern prevents race conditions and simplifies reasoning about state changes.
pub struct AppState {
    // Input mode
    /// What the shared single-line text input is currently capturing. Covers the
    /// old `editing_nickname`/`creating_group` booleans: `Chat` = normal
    /// input, [`p2p_app::InputMode::EditingNickname`] = nickname edit,
    /// [`p2p_app::InputMode::CreatingGroup`] = group name entry.
    pub input_mode: p2p_app::InputMode,

    // Runtime environment (TUI input/output flags)
    /// Terminal input/output environment flags (see the individual docs).
    pub env: TerminalEnv,

    // Scroll preferences (Chat + Log)
    /// Per-tab auto-scroll toggles (broadcast chat + log tabs).
    pub auto_scroll: p2p_app::AutoScrollFlags,
    // Messages & Chat
    pub messages: VecDeque<p2p_app::DisplayMessage>,
    // Message IDs aligned with `messages` (used for receipts / click actions).
    pub message_ids: VecDeque<Option<String>>,
    // DB row IDs aligned with `messages`/`message_ids`; `Some(id)` only for
    // rows we sent (US-005). Drives the local-only `Delete` binding.
    pub message_db_ids: VecDeque<Option<i32>>,
    // Broadcast receipts: msg_id -> (peer_id -> received_at epoch seconds).
    pub broadcast_receipts: HashMap<String, HashMap<String, f64>>,
    pub dm_messages: HashMap<String, VecDeque<String>>,
    // DM message IDs aligned with dm_messages[peer_id].
    pub dm_message_ids: HashMap<String, VecDeque<Option<String>>>,
    // DB row IDs aligned with dm_messages[peer_id] / dm_message_ids[peer_id].
    pub dm_message_db_ids: HashMap<String, VecDeque<Option<i32>>>,
    // DM receipts: msg_id -> (peer_id, received_at epoch seconds).
    pub dm_receipts: HashMap<String, (String, f64)>,

    // Peer Management
    pub peers: VecDeque<p2p_app::PeerRecord>,
    // Canonical Contacts-tab rows (from `contacts::build_contact_rows`).
    // Rebuilt whenever the device list changes (see `rebuild_contacts`); the
    // live `peers` deque stays the single device source, and grouped users
    // collapse here into one row per contact.
    pub contacts: Vec<p2p_app::contacts::ContactRow>,
    // The unfiltered, sorted Contacts rows: `contacts` is this list with the
    // active search query applied, so the table, click mapping, selection and
    // scroll math all agree on one filtered list.
    pub contacts_all: Vec<p2p_app::contacts::ContactRow>,
    // Active Contacts-tab search query (empty = no filter). Case-insensitive
    // substring match over the contact/device ids and display names.
    pub contact_search: String,
    // Contact IDs expanded into their device sub-rows on the Contacts tab.
    pub expanded_contact_ids: HashSet<String>,
    // Peers the operator muted (US-054). The swarm boundary drops their
    // traffic and refuses to dial them; this set only drives the UI state.
    pub blocked_peers: HashSet<String>,
    pub concurrent_peers: usize,
    // How many broadcasts *we* sent to each peer (bulk-loaded from the
    // `broadcast_recipients` table via `messages::get_all_peer_stats`, refreshed
    // whenever peers change). Drives the peers-table "Broadcast" column.
    pub broadcast_sent_to_peer: HashMap<String, usize>,
    // Currently connected peers (shared connected-tracking logic with the
    // Flutter backend). Mirror of Flutter's "Connected peers" on Settings.
    pub connected: p2p_app::connected::ConnectedTracker,
    pub local_nicknames: HashMap<String, String>,
    pub received_nicknames: HashMap<String, String>,
    // Per-peer self nickname override: peer_id -> nickname we present to that peer.
    pub self_nicknames_for_peers: HashMap<String, String>,

    // Groups
    pub group_summaries: Vec<p2p_app::groups::GroupSummary>,
    pub group_messages: HashMap<String, VecDeque<String>>,
    // Group message IDs aligned with group_messages[group_id].
    pub group_message_ids: HashMap<String, VecDeque<Option<String>>>,
    // DB row IDs aligned with group_messages[group_id] / group_message_ids[group_id].
    pub group_message_db_ids: HashMap<String, VecDeque<Option<i32>>>,
    // Sender peer ID for each group message (None = sent by local user).
    pub group_message_peer_ids: HashMap<String, VecDeque<Option<String>>>,
    // Scroll state per group chat: group_id -> (scroll_offset, auto_scroll).
    pub group_scroll_state: HashMap<String, (usize, bool)>,
    // Selected group in the Groups list tab.
    pub group_selection: usize,

    // UI State (TUI-specific)
    pub active_tab: usize,
    pub dynamic_tabs: DynamicTabs,
    pub chat_input: TextArea<'static>,
    pub peer_selection: usize,     // For navigating peer list
    pub peer_sort_column: usize, // Active sort column (0=Name,1=DM,2=Broadcast,3=Last Seen,4=First Seen)
    pub peer_sort_ascending: bool, // Whether the peer list is sorted ascending on the active column
    pub peer_table_offset: usize, // First visible row of the peer table (set by render loop)
    pub last_mouse_row: u16,     // For mouse-targeted scroll behavior in split layouts

    // Scroll State (Chat tab)
    pub chat_scroll_offset: usize,
    pub chat_unread_count: usize, // Unread broadcast messages while scrolled up
    pub chat_area_height: usize,  // Height of message area in rows (set by render loop)
    pub terminal_width: usize,    // Terminal width in columns (set by render loop)

    // Scroll State (Log tab)
    pub log_scroll_offset: usize,

    // Per-DM scroll state: peer_id -> (scroll_offset, auto_scroll)
    pub dm_scroll_state: HashMap<String, (usize, bool)>,
    // Per-DM broadcast scroll state: peer_id -> (scroll_offset, auto_scroll)
    pub dm_broadcast_scroll_state: HashMap<String, (usize, bool)>,

    // Runtime Context
    pub own_nickname: String,
    pub local_peer_id: String,
    pub topic_str: String,

    // Edit Mode
    /// Peer the nickname editor targets: the peer whose *local* display name is
    /// being set (with [`p2p_app::InputMode::EditingLocalNickname`]) or the peer
    /// we announce a personalised nickname to (with
    /// [`p2p_app::InputMode::EditingNickname`]); `None` in the nickname mode
    /// means this install's own nickname.
    pub editing_nickname_peer: Option<String>,

    // Ad-hoc UI popup (used for receipt timing details, etc.)
    pub popup: Option<String>,

    /// Per-conversation reply drafts (US-006). Keyed by `chat`, `dm:<peer>` or
    /// `group:<id>` so a draft survives switching tabs but never leaks into a
    /// different conversation. `Ctrl+R` captures the newest row; sending or
    /// `Esc` consumes/clears it.
    pub reply_drafts: HashMap<String, p2p_app::ReplyRef>,

    /// Incoming device-binding intro awaiting the account holder's decision
    /// (TUI user decision: like the mobile dialog, accepting is explicit — a
    /// confirmed keypress, never automatic). Rendered as a popup; 'a' accepts,
    /// 'd'/Esc declines.
    pub device_binding_prompt: Option<DeviceBindingPrompt>,

    /// Group the user asked to leave, awaiting confirmation ('y'/Enter
    /// confirms, any other key or Esc cancels). Single-slot like the
    /// device-binding prompt; leaving is local-only (the group row, its
    /// messages and the membership are deleted here, other members keep their
    /// copy), mirroring Flutter's "Leave group" dialog.
    pub group_leave_prompt: Option<GroupLeavePrompt>,

    /// A sent message the user asked to hide locally, awaiting confirmation
    /// ('y'/Enter confirms; any other key or Esc cancels). Single-slot like the
    /// other prompts: deletion is local-only, so no swarm command is queued.
    pub delete_message_prompt: Option<DeleteMessagePrompt>,

    /// Live call session (calls lane). One session at a time, mirroring the
    /// single-slot device-binding prompt; `None` = no active/ringing call.
    /// Advanced here on call events; the interactive surface (accept/reject/
    /// hangup keys + prompt overlay + status line) lands in the TUI slice.
    pub call_session: Option<p2p_app::CallSession>,

    // Settings tab (read-only node/network diagnostics)
    pub db_url: String,
    pub platform: String,
    pub network_size: String,

    /// Persisted bootstrap/relay multiaddrs (Settings tab), loaded at startup
    /// and after every add/remove. The node reads this list once at start.
    pub network_peers: Vec<p2p_app::network_config::NetworkPeer>,
    /// Highlighted row in the Settings network-peers list (`b`/`r` add,
    /// `x` removes the highlighted entry).
    pub network_peer_selection: usize,
    /// Kind captured by the active add prompt
    /// ([`p2p_app::InputMode::AddingNetworkPeer`]); `None` otherwise.
    pub network_peer_kind: Option<p2p_app::network_config::NetworkPeerKind>,

    /// This install's user ID (None before provisioning) — Settings tab.
    pub own_user_id: Option<String>,
    /// Devices bound into this install's user: `(peer_id, bound_at)` —
    /// Settings tab.
    pub own_user_devices: Vec<(String, String)>,
}

#[derive(Clone)]
pub struct DeviceBindingPrompt {
    pub peer_id: String,
    pub device_pubkey: Vec<u8>,
}

/// A group the user asked to leave, pending a `y`/Enter confirmation.
#[derive(Clone)]
pub struct GroupLeavePrompt {
    pub group_id: String,
    pub display_name: String,
}

/// A sent message the user asked to hide locally (US-005), pending confirmation.
///
/// The tab identifies the conversation whose aligned queues hold the row, the
/// id selects it in the DB, and the preview is shown in the confirmation popup.
#[derive(Clone, Debug)]
pub struct DeleteMessagePrompt {
    pub message_id: i32,
    pub is_group: bool,
    pub preview: String,
    pub tab_content: p2p_app::tui_tabs::TabContent,
}

/// Index + DB id of the newest outgoing row in an aligned id queue.
fn last_sent_db_id(ids: &VecDeque<Option<i32>>) -> Option<(usize, i32)> {
    ids.iter()
        .enumerate()
        .rev()
        .find_map(|(idx, id)| id.map(|id| (idx, id)))
}

/// Position of a specific DB id in an aligned queue (newest wins).
fn index_of_db_id(ids: &VecDeque<Option<i32>>, id: i32) -> Option<usize> {
    ids.iter().rposition(|slot| *slot == Some(id))
}

/// Short single-line preview for a confirmation popup. Quoted headers are
/// skipped so the popup previews the message body, not its reply banner.
fn short_preview(text: &str) -> String {
    let trimmed = text.rsplit('\n').next().unwrap_or(text).trim();
    let mut preview: String = trimmed.chars().take(72).collect();
    if trimmed.chars().count() > 72 {
        preview.push('\u{2026}');
    }
    preview
}

impl AppState {
    /// True while the shared text input is capturing a nickname edit (either
    /// this install's own nickname, a per-peer announced nickname, or a peer's
    /// local display name).
    pub const fn is_editing_nickname(&self) -> bool {
        matches!(
            self.input_mode,
            p2p_app::InputMode::EditingNickname | p2p_app::InputMode::EditingLocalNickname
        )
    }

    /// True while the editor is setting the local display name of
    /// [`Self::editing_nickname_peer`].
    pub const fn is_editing_local_nickname(&self) -> bool {
        matches!(self.input_mode, p2p_app::InputMode::EditingLocalNickname)
    }

    /// True while the shared text input is capturing a group name.
    pub const fn is_creating_group(&self) -> bool {
        matches!(self.input_mode, p2p_app::InputMode::CreatingGroup)
    }
    #[must_use]
    pub const fn is_dialing_peer(&self) -> bool {
        matches!(self.input_mode, p2p_app::InputMode::DialingPeer)
    }

    pub fn cancel_nickname_edit(&mut self) {
        if !self.is_editing_nickname() {
            return;
        }
        self.input_mode = p2p_app::InputMode::Chat;
        self.editing_nickname_peer = None;
        self.chat_input = TextArea::default();
    }

    /// Open the shared editor for this install's own nickname (Settings tab).
    pub fn begin_self_nickname_edit(&mut self) {
        self.input_mode = p2p_app::InputMode::EditingNickname;
        self.editing_nickname_peer = None;
        self.chat_input = TextArea::default();
    }

    /// Open the shared editor for the local display name of `peer_id` (the name
    /// only this install shows). The editor starts empty, like the self-nickname
    /// one; canceling leaves the stored nickname untouched.
    pub fn begin_local_nickname_edit(&mut self, peer_id: String) {
        self.input_mode = p2p_app::InputMode::EditingLocalNickname;
        self.editing_nickname_peer = Some(peer_id);
        self.chat_input = TextArea::default();
    }

    /// Cancel an in-progress "create/join group" name entry.
    pub fn cancel_group_create(&mut self) {
        if !self.is_creating_group() {
            return;
        }
        self.input_mode = p2p_app::InputMode::Chat;
        self.chat_input = TextArea::default();
    }

    /// Begin capturing a peer multiaddr to dial (manual peer address entry).
    pub fn begin_dial_peer(&mut self) {
        self.input_mode = p2p_app::InputMode::DialingPeer;
        self.chat_input = TextArea::default();
    }

    /// Cancel an in-progress dial entry.
    pub fn cancel_dial_peer(&mut self) {
        if !self.is_dialing_peer() {
            return;
        }
        self.input_mode = p2p_app::InputMode::Chat;
        self.chat_input = TextArea::default();
    }

    /// True while the shared input captures a bootstrap/relay multiaddr.
    #[must_use]
    pub const fn is_adding_network_peer(&self) -> bool {
        matches!(self.input_mode, p2p_app::InputMode::AddingNetworkPeer)
    }

    /// Begin the Settings "add network peer" prompt for `kind`.
    pub fn begin_add_network_peer(&mut self, kind: p2p_app::network_config::NetworkPeerKind) {
        self.input_mode = p2p_app::InputMode::AddingNetworkPeer;
        self.network_peer_kind = Some(kind);
        self.chat_input = TextArea::default();
    }

    /// Cancel an in-progress network-peer entry.
    pub fn cancel_add_network_peer(&mut self) {
        if !self.is_adding_network_peer() {
            return;
        }
        self.input_mode = p2p_app::InputMode::Chat;
        self.network_peer_kind = None;
        self.chat_input = TextArea::default();
    }

    /// Reload the persisted network-peer list, clamping the selection.
    ///
    /// Best-effort: a DB error leaves the last known list in place (the node
    /// still starts with the env-var fallbacks).
    pub fn reload_network_peers(&mut self) {
        if let Ok(peers) = p2p_app::network_config::list_network_peers() {
            self.network_peers = peers;
        }
        self.network_peer_selection = self
            .network_peer_selection
            .min(self.network_peers.len().saturating_sub(1));
    }

    /// Move the network-peers highlight with the Settings Up/Down/Home/End keys.
    pub fn move_network_peer_selection(&mut self, key: crossterm::event::KeyCode) {
        let len = self.network_peers.len();
        if len == 0 {
            self.network_peer_selection = 0;
            return;
        }
        let last = len.saturating_sub(1);
        self.network_peer_selection = match key {
            crossterm::event::KeyCode::Up => self.network_peer_selection.saturating_sub(1),
            crossterm::event::KeyCode::Down => {
                self.network_peer_selection.saturating_add(1).min(last)
            }
            crossterm::event::KeyCode::Home | crossterm::event::KeyCode::PageUp => 0,
            crossterm::event::KeyCode::End | crossterm::event::KeyCode::PageDown => last,
            _ => self.network_peer_selection,
        };
    }

    /// Arm the leave-group confirmation for `group_id` (single slot: an
    /// already-armed prompt is replaced).
    pub fn begin_group_leave(&mut self, group_id: String, display_name: String) {
        self.group_leave_prompt = Some(GroupLeavePrompt {
            group_id,
            display_name,
        });
    }

    /// Clear the leave-group confirmation; true when one was armed.
    pub fn cancel_group_leave(&mut self) -> bool {
        self.group_leave_prompt.take().is_some()
    }

    /// Arm the local-delete confirmation for a sent message in the conversation
    /// shown by `tab_content` (single slot: a previous prompt is replaced).
    pub fn begin_delete_message(
        &mut self,
        message_id: i32,
        is_group: bool,
        preview: String,
        tab_content: p2p_app::tui_tabs::TabContent,
    ) {
        self.delete_message_prompt = Some(DeleteMessagePrompt {
            message_id,
            is_group,
            preview,
            tab_content,
        });
    }

    /// Clear the local-delete confirmation; true when one was armed.
    pub fn cancel_delete_message(&mut self) -> bool {
        self.delete_message_prompt.take().is_some()
    }

    /// The most recent outgoing message in the conversation shown by `tab`,
    /// as `(db_id, short_preview)`; `None` when there is nothing deletable.
    #[must_use]
    pub fn last_sent_message(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<(i32, String)> {
        use p2p_app::tui_tabs::TabContent;

        let (id, text) = match tab {
            TabContent::Chat => {
                let (idx, id) = last_sent_db_id(&self.message_db_ids)?;
                (id, self.messages.get(idx).map_or("", |m| m.text.as_str()))
            }
            TabContent::Direct(peer_id) => {
                let (idx, id) = last_sent_db_id(self.dm_message_db_ids.get(peer_id)?)?;
                (
                    id,
                    self.dm_messages
                        .get(peer_id)
                        .and_then(|msgs| msgs.get(idx))
                        .map_or("", String::as_str),
                )
            }
            TabContent::GroupChat(group_id) => {
                let (idx, id) = last_sent_db_id(self.group_message_db_ids.get(group_id)?)?;
                (
                    id,
                    self.group_messages
                        .get(group_id)
                        .and_then(|msgs| msgs.get(idx))
                        .map_or("", String::as_str),
                )
            }
            _ => return None,
        };
        Some((id, short_preview(text)))
    }

    /// Key of the reply draft belonging to a tab. `None` for tabs without a
    /// message composer, so a draft can never leak into another conversation.
    #[must_use]
    pub fn reply_key(tab: &p2p_app::tui_tabs::TabContent) -> Option<String> {
        use p2p_app::tui_tabs::TabContent;
        match tab {
            TabContent::Chat => Some("chat".to_string()),
            TabContent::Direct(peer_id) => Some(format!("dm:{peer_id}")),
            TabContent::GroupChat(group_id) => Some(format!("group:{group_id}")),
            _ => None,
        }
    }

    /// Capture the newest replyable message in `tab` as a reply draft.
    ///
    /// The source must have a stable wire id; otherwise the quote could not be
    /// correlated by the receiver. Attached files render as their canonical
    /// chip text, which is already part of the transcript line.
    pub fn begin_reply(
        &mut self,
        tab: &p2p_app::tui_tabs::TabContent,
    ) -> Option<p2p_app::ReplyRef> {
        use p2p_app::tui_tabs::TabContent;

        let key = Self::reply_key(tab)?;
        let (line, id) = match tab {
            TabContent::Chat => {
                let idx = self
                    .message_ids
                    .iter()
                    .rposition(|id| id.as_deref().is_some_and(|id| !id.trim().is_empty()))?;
                (
                    self.messages.get(idx)?.text.as_str(),
                    self.message_ids.get(idx)?.as_deref()?,
                )
            }
            TabContent::Direct(peer_id) => {
                let ids = self.dm_message_ids.get(peer_id)?;
                let idx = ids
                    .iter()
                    .rposition(|id| id.as_deref().is_some_and(|id| !id.trim().is_empty()))?;
                (
                    self.dm_messages.get(peer_id)?.get(idx)?.as_str(),
                    ids.get(idx)?.as_deref()?,
                )
            }
            TabContent::GroupChat(group_id) => {
                let ids = self.group_message_ids.get(group_id)?;
                let idx = ids
                    .iter()
                    .rposition(|id| id.as_deref().is_some_and(|id| !id.trim().is_empty()))?;
                (
                    self.group_messages.get(group_id)?.get(idx)?.as_str(),
                    ids.get(idx)?.as_deref()?,
                )
            }
            _ => return None,
        };
        let reply = p2p_app::reply::from_display_line(line, id)?;
        self.reply_drafts.insert(key, reply.clone());
        Some(reply)
    }

    /// Draft currently attached to `tab`, if any.
    #[must_use]
    pub fn reply_draft(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<&p2p_app::ReplyRef> {
        let key = Self::reply_key(tab)?;
        self.reply_drafts.get(key.as_str())
    }

    /// Consume the draft for `tab` and return it (used just before sending).
    pub fn take_reply(&mut self, tab: &p2p_app::tui_tabs::TabContent) -> Option<p2p_app::ReplyRef> {
        let key = Self::reply_key(tab)?;
        self.reply_drafts.remove(key.as_str())
    }

    /// Clear the draft for `tab`; true when one was armed.
    pub fn clear_reply(&mut self, tab: &p2p_app::tui_tabs::TabContent) -> bool {
        let Some(key) = Self::reply_key(tab) else {
            return false;
        };
        self.reply_drafts.remove(key.as_str()).is_some()
    }

    /// Remove the prompted row from the conversation's aligned queues after the
    /// DB tombstone succeeded. Only local state changes — no swarm command.
    /// Returns false (leaving state untouched) if the row is no longer present.
    pub fn remove_deleted_message(&mut self, prompt: &DeleteMessagePrompt) -> bool {
        use p2p_app::tui_tabs::TabContent;

        match &prompt.tab_content {
            TabContent::Chat => {
                let Some(idx) = index_of_db_id(&self.message_db_ids, prompt.message_id) else {
                    return false;
                };
                let _ = self.message_db_ids.remove(idx);
                let _ = self.message_ids.remove(idx);
                let _ = self.messages.remove(idx);
                self.chat_scroll_offset = self.chat_scroll_offset.min(self.messages.len());
                true
            }
            TabContent::Direct(peer_id) => {
                let Some(ids) = self.dm_message_db_ids.get_mut(peer_id) else {
                    return false;
                };
                let Some(idx) = index_of_db_id(ids, prompt.message_id) else {
                    return false;
                };
                let _ = ids.remove(idx);
                if let Some(ids) = self.dm_message_ids.get_mut(peer_id) {
                    let _ = ids.remove(idx);
                }
                if let Some(msgs) = self.dm_messages.get_mut(peer_id) {
                    let _ = msgs.remove(idx);
                    if let Some(offset) = self.dm_scroll_state.get_mut(peer_id) {
                        offset.0 = offset.0.min(msgs.len());
                    }
                }
                true
            }
            TabContent::GroupChat(group_id) => {
                let Some(ids) = self.group_message_db_ids.get_mut(group_id) else {
                    return false;
                };
                let Some(idx) = index_of_db_id(ids, prompt.message_id) else {
                    return false;
                };
                let _ = ids.remove(idx);
                if let Some(ids) = self.group_message_ids.get_mut(group_id) {
                    let _ = ids.remove(idx);
                }
                if let Some(msgs) = self.group_messages.get_mut(group_id) {
                    let _ = msgs.remove(idx);
                }
                if let Some(ids) = self.group_message_peer_ids.get_mut(group_id) {
                    let _ = ids.remove(idx);
                }
                if let Some(msgs) = self.group_messages.get(group_id)
                    && let Some(scroll) = self.group_scroll_state.get_mut(group_id)
                {
                    scroll.0 = scroll.0.min(msgs.len());
                }
                true
            }
            _ => false,
        }
    }

    /// Apply the local side of leaving `group_id` after the DB rows are gone:
    /// refresh the list, keep the selection in range, drop the in-memory
    /// history (so re-joining starts from the backfill, not stale rows), and
    /// close the group's chat tab (adjusting the active tab if it was open).
    pub fn apply_group_leave(&mut self, group_id: &str) {
        self.reload_group_summaries();
        self.group_selection = self
            .group_selection
            .min(self.group_summaries.len().saturating_sub(1));
        self.group_messages.remove(group_id);
        self.group_message_ids.remove(group_id);
        self.group_message_db_ids.remove(group_id);
        self.group_message_peer_ids.remove(group_id);
        self.group_scroll_state.remove(group_id);
        self.reply_drafts.remove(&format!("group:{group_id}"));
        if let Some(closed) = self.dynamic_tabs.remove_group_tab(group_id) {
            if closed == self.active_tab {
                // Leaving from the group's own chat tab: back to the list.
                self.active_tab = p2p_app::tui_tabs::FIXED_TAB_COUNT.saturating_sub(1);
            } else if closed < self.active_tab {
                self.active_tab = self.active_tab.saturating_sub(1);
            }
        }
    }

    /// Refresh the group list from the database (used after create/join and on
    /// incoming group messages so member counts stay current).
    pub fn reload_group_summaries(&mut self) {
        if let Ok(summaries) = p2p_app::groups::list_groups_with_member_counts() {
            self.group_summaries = summaries;
        }
    }

    /// Re-resolve this install's user identity for the Settings tab: our
    /// `user_id` plus the devices bound into it. Called at startup right after
    /// auto-provisioning and after any claim exchange that changes the local
    /// user's device-set (accepting an intro binds a foreign device into our
    /// user; a `DeviceBindingReceived` binds us into a foreign user).
    pub fn refresh_my_user(&mut self) {
        self.own_user_id = p2p_app::identity::my_user_id().ok().flatten();
        self.own_user_devices = match p2p_app::identity::my_user_devices() {
            Ok(devices) => devices
                .into_iter()
                .map(|(peer_id, bound_at)| (peer_id, p2p_app::format_peer_datetime(bound_at)))
                .collect(),
            Err(e) => {
                p2p_app::p2plog_debug(format!("Failed to refresh own user identity: {e}"));
                Vec::new()
            }
        };
    }

    /// Re-sort the Contacts tab: refresh the per-contact stats, rebuild the
    /// contact rows from the live peer list, and order them by the active
    /// column/order while keeping the selected contact (by id) selected. A
    /// device-row selection collapses onto its contact row here.
    pub fn resort_peers(&mut self) {
        self.broadcast_sent_to_peer =
            p2p_app::messages::get_all_peer_stats().map_or_default(|stats| {
                stats
                    .into_iter()
                    .map(|(pid, s)| (pid, usize::try_from(s.broadcast_sent_to_peer).unwrap_or(0)))
                    .collect()
            });
        self.rebuild_contacts();
        self.contacts = p2p_app::contacts::sort_canonical_rows(
            &self.contacts,
            self.peer_sort_column,
            self.peer_sort_ascending,
        );
        self.contacts_all.clone_from(&self.contacts);
        self.apply_contact_search();
    }

    /// Re-derive [`Self::contacts`] from the unfiltered [`Self::contacts_all`]
    /// with the active search query, keeping the currently selected contact
    /// selected when it survives the filter (otherwise the selection falls back
    /// to the first row).
    pub fn apply_contact_search(&mut self) {
        let previous = self.selected_contact_id();
        if self.contact_search.is_empty() {
            self.contacts.clone_from(&self.contacts_all);
        } else {
            let query = self.contact_search.to_lowercase();
            self.contacts = self
                .contacts_all
                .iter()
                .filter(|c| contact_matches(c, &query))
                .cloned()
                .collect();
        }
        self.peer_selection = previous
            .and_then(|contact_id| {
                self.contacts
                    .iter()
                    .position(|c| c.contact_id == contact_id)
            })
            .unwrap_or(0)
            .min(self.contacts.len().saturating_sub(1));
    }

    /// Set the Contacts-tab search query and re-filter the table.
    pub fn set_contact_search(&mut self, query: &str) {
        self.contact_search = query.to_string();
        self.apply_contact_search();
    }

    /// True while the shared text input is capturing a Contacts search query.
    pub const fn is_searching_contacts(&self) -> bool {
        matches!(self.input_mode, p2p_app::InputMode::SearchingContacts)
    }

    /// Open the Contacts search box, pre-filled with the active query so it can
    /// be edited (clearing it and confirming restores the full list).
    pub fn begin_contact_search(&mut self) {
        self.input_mode = p2p_app::InputMode::SearchingContacts;
        let mut input = TextArea::default();
        input.insert_str(&self.contact_search);
        self.chat_input = input;
    }

    /// Leave the search box; `keep_query` keeps the filter applied (Enter) or
    /// clears it and restores the full list (Esc).
    pub fn end_contact_search(&mut self, keep_query: bool) {
        if !self.is_searching_contacts() {
            return;
        }
        self.input_mode = p2p_app::InputMode::Chat;
        if !keep_query {
            self.set_contact_search("");
        }
        self.chat_input = TextArea::default();
    }

    /// The selected row of the flattened Contacts table, in the contact list's
    /// display order (contact rows plus any expanded device rows).
    pub fn selected_contact_ref(&self) -> Option<p2p_app::tui_helpers::ContactTableRowRef> {
        let plan =
            p2p_app::tui_helpers::contact_table_plan(&self.contacts, &self.expanded_contact_ids);
        plan.get(self.peer_selection).copied()
    }

    /// The peer ID the current selection targets: the contact's best device, or
    /// the exact device of a selected device row.
    pub fn selected_contact_peer_id(&self) -> Option<String> {
        self.selected_contact_ref()
            .and_then(|row| row.peer_id(&self.contacts).map(str::to_owned))
    }

    /// The contact ID owning the current selection (used to preserve the
    /// selection across re-sorts).
    pub fn selected_contact_id(&self) -> Option<String> {
        self.selected_contact_ref()
            .and_then(|row| row.contact_id(&self.contacts).map(str::to_owned))
    }

    /// Expand/collapse the selected contact's device rows. No-op on a
    /// single-device contact or a device row.
    pub fn toggle_expand_selected_contact(&mut self) {
        let Some(p2p_app::tui_helpers::ContactTableRowRef::Contact(idx)) =
            self.selected_contact_ref()
        else {
            return;
        };
        let Some(contact) = self.contacts.get(idx) else {
            return;
        };
        if contact.device_count <= 1 {
            return;
        }
        let contact_id = contact.contact_id.clone();
        if self.expanded_contact_ids.contains(&contact_id) {
            self.expanded_contact_ids.remove(&contact_id);
        } else {
            self.expanded_contact_ids.insert(contact_id);
        }
    }

    /// Rebuild `self.contacts` from the live peer list through the shared
    /// grouping builder (`contacts::build_contact_rows`).
    ///
    /// The grouping/aggregation is the same pure logic the DB loader and the
    /// Flutter binding use; only the device inputs (and the display-name
    /// resolution) come from the live in-memory state, so the Contacts tab
    /// never regresses peers that haven't been persisted yet. Identity lookups
    /// are defensive: when they fail (or the DB is unreachable) every device
    /// stays an unbound single-device row — exactly the pre-contacts rendering.
    fn rebuild_contacts(&mut self) {
        let stats: HashMap<String, PeerMessageStats> = self
            .peers
            .iter()
            .map(|p| {
                (
                    p.peer_id.clone(),
                    PeerMessageStats {
                        dm_count: i64::try_from(self.dm_messages.dm_count_for(&p.peer_id))
                            .unwrap_or(0),
                        broadcast_sent_to_peer: i64::try_from(
                            self.broadcast_sent_to_peer
                                .get(&p.peer_id)
                                .copied()
                                .unwrap_or(0),
                        )
                        .unwrap_or(0),
                    },
                )
            })
            .collect();

        let devices: Vec<ContactDeviceInput> = self
            .peers
            .iter()
            .map(|p| ContactDeviceInput {
                peer_id: p.peer_id.clone(),
                first_seen: p.first_seen.clone(),
                last_seen: p.last_seen.clone(),
                received_nickname: self.received_nicknames.get(&p.peer_id).cloned(),
                local_nickname: self.local_nicknames.get(&p.peer_id).cloned(),
            })
            .collect();

        let device_display_names: HashMap<String, String> = self
            .peers
            .iter()
            .map(|p| {
                (
                    p.peer_id.clone(),
                    p2p_app::get_peer_display_name_or_short(&p.peer_id),
                )
            })
            .collect();

        let peer_ids: Vec<String> = self.peers.iter().map(|p| p.peer_id.clone()).collect();
        let peer_user: HashMap<String, String> =
            p2p_app::identity::find_user_ids_for_peers(&peer_ids).unwrap_or_default();
        let mut user_ids: Vec<String> = peer_user.values().cloned().collect();
        user_ids.sort();
        user_ids.dedup();
        let users = p2p_app::identity::find_users(&user_ids).unwrap_or_default();
        let overrides: HashMap<String, String> = p2p_app::get_local_peer_id()
            .ok()
            .and_then(|id| {
                p2p_app::identity::find_user_id_for_peer(&id.to_string())
                    .ok()
                    .flatten()
            })
            .and_then(|owner| p2p_app::identity::list_contacts(&owner).ok())
            .map_or_default(|contacts| {
                contacts
                    .into_iter()
                    .filter_map(|c| c.nickname_override.map(|n| (c.contact_user_id, n)))
                    .collect()
            });

        self.contacts = p2p_app::contacts::build_contact_rows(
            &devices,
            &device_display_names,
            &peer_user,
            &users,
            &overrides,
            &stats,
        );
    }

    pub fn new(
        topic_str: String,
        own_nickname: String,
        local_peer_id: String,
        local_nicknames: HashMap<String, String>,
        received_nicknames: HashMap<String, String>,
        self_nicknames_for_peers: HashMap<String, String>,
        data: InitialStateData,
    ) -> Self {
        let mut state = Self {
            messages: data.messages,
            message_ids: data.message_ids,
            message_db_ids: data.message_db_ids,
            broadcast_receipts: data.broadcast_receipts,
            dm_messages: HashMap::new(),
            dm_message_ids: HashMap::new(),
            dm_message_db_ids: HashMap::new(),
            dm_receipts: data.dm_receipts,
            peers: data.peers,
            contacts: Vec::new(),
            contacts_all: Vec::new(),
            contact_search: String::new(),
            expanded_contact_ids: HashSet::new(),
            blocked_peers: HashSet::new(),
            broadcast_sent_to_peer: HashMap::new(),
            group_summaries: data.groups,
            group_messages: HashMap::new(),
            group_message_ids: HashMap::new(),
            group_message_db_ids: HashMap::new(),
            group_message_peer_ids: HashMap::new(),
            group_scroll_state: HashMap::new(),
            group_selection: 0,
            input_mode: p2p_app::InputMode::Chat,
            dynamic_tabs: DynamicTabs::new(),
            active_tab: 0,
            chat_input: TextArea::default(),
            peer_selection: 0,
            peer_sort_column: 3,
            peer_sort_ascending: false,
            peer_table_offset: 0,
            env: TerminalEnv {
                kitty_keyboard_active: true,
                mouse_capture: true,
                peer_names_warmed: false,
            },
            concurrent_peers: 0,
            connected: p2p_app::connected::ConnectedTracker::new(),
            last_mouse_row: 0,
            chat_scroll_offset: 0,
            auto_scroll: p2p_app::AutoScrollFlags {
                chat: true,
                log: true,
            },
            chat_unread_count: 0,
            chat_area_height: 0,
            terminal_width: 0,
            log_scroll_offset: 0,
            dm_scroll_state: HashMap::new(),
            dm_broadcast_scroll_state: HashMap::new(),
            own_nickname,
            local_peer_id,
            local_nicknames,
            received_nicknames,
            self_nicknames_for_peers,
            topic_str,
            editing_nickname_peer: None,
            popup: None,
            reply_drafts: HashMap::new(),
            device_binding_prompt: None,
            group_leave_prompt: None,
            delete_message_prompt: None,
            call_session: None,
            db_url: String::new(),
            platform: String::new(),
            network_size: String::new(),
            network_peers: Vec::new(),
            network_peer_selection: 0,
            network_peer_kind: None,
            own_user_id: None,
            own_user_devices: Vec::new(),
        };
        state.reload_blocked_peers();
        state.rebuild_contacts();
        state.contacts_all.clone_from(&state.contacts);
        state
    }

    /// Refresh the muted-peer set from the `blocked_peers` table (US-054).
    ///
    /// Best-effort, like the blocklist itself: an unreadable database yields an
    /// empty set, so a broken DB never makes the UI claim peers are muted.
    pub fn reload_blocked_peers(&mut self) {
        self.blocked_peers = p2p_app::blocklist::list_blocked_peers()
            .unwrap_or_default()
            .into_iter()
            .collect();
    }
}

type FormattedMessages = (
    VecDeque<p2p_app::DisplayMessage>,
    VecDeque<Option<String>>,
    VecDeque<Option<i32>>,
);

/// Pure: formats DB messages into display-ready rows plus their aligned wire
/// IDs and DB row IDs (`Some` only when the row is one we sent, US-005).
///
/// Separated from the DB call so it can be unit-tested without a database.
fn format_messages_from_db(
    db_messages: &[p2p_app::generated::models_queryable::Message],
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
    own_nickname: &str,
) -> FormattedMessages {
    let mut messages = VecDeque::new();
    let mut message_ids = VecDeque::new();
    let mut message_db_ids = VecDeque::new();
    for msg in db_messages.iter().rev() {
        let ts = p2p_app::format_peer_datetime(msg.created_at);
        let sender = match (msg.sender_nickname.as_ref(), msg.peer_id.as_ref()) {
            (Some(nick), _) => format!("[{nick}]"),
            (None, Some(pid)) => {
                let display = p2p_app::peer_display_name(pid, local_nicknames, received_nicknames);
                format!("[{display}]")
            }
            (None, None) => format!("[{own_nickname}]"),
        };
        let body = p2p_app::files::render_stored_body(&msg.content, msg.attachment_json.as_deref());
        let reply = msg
            .reply_to_json
            .as_deref()
            .and_then(p2p_app::reply::decode);
        messages.push_back(p2p_app::DisplayMessage {
            text: p2p_app::reply::with_quoted_header(
                format!("{ts} {sender} {body}"),
                reply.as_ref(),
            ),
            sender_peer_id: msg.peer_id.clone(),
        });
        message_ids.push_back(msg.msg_id.clone());
        message_db_ids.push_back((msg.sent == 1).then_some(msg.id));
    }
    (messages, message_ids, message_db_ids)
}

pub fn load_and_format_messages(
    topic_str: &str,
    max_messages: usize,
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
    own_nickname: &str,
) -> FormattedMessages {
    p2p_app::load_messages(topic_str, max_messages).map_or_else(
        |_| {
            p2p_app::p2plog_debug("Failed to load messages from database");
            (VecDeque::new(), VecDeque::new(), VecDeque::new())
        },
        |db_messages| {
            format_messages_from_db(
                &db_messages,
                local_nicknames,
                received_nicknames,
                own_nickname,
            )
        },
    )
}

/// Does a contact row match a Contacts-tab search query?
///
/// Case-insensitive substring match against the contact's id and display name
/// plus every device's peer id and display name — so searching a peer ID finds
/// the row even when the contact is shown by a grouped user name.
#[must_use]
pub fn contact_matches(contact: &p2p_app::contacts::ContactRow, query_lower: &str) -> bool {
    contact.contact_id.to_lowercase().contains(query_lower)
        || contact.display_name.to_lowercase().contains(query_lower)
        || contact.device_rows.iter().any(|d| {
            d.peer_id.to_lowercase().contains(query_lower)
                || d.display_name.to_lowercase().contains(query_lower)
        })
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_state.rs"]
mod tests;
