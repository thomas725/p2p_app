use super::call_audio::TuiAudioHandle;
use super::{DynamicTabs, TextArea};
use p2p_app::contacts::ContactDeviceInput;
use p2p_app::messages::PeerMessageStats;
use p2p_app::tui_helpers::PeerMessageMap;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Channel capacity for inter-task communication (events per task)
pub const CHANNEL_CAPACITY: usize = 100;
/// Maximum direct messages to keep per peer conversation.
pub use p2p_app::types::MAX_DM_HISTORY;
/// Maximum messages to keep in memory (older messages are dropped).
pub use p2p_app::types::MAX_MESSAGE_HISTORY;
/// Input poll interval in milliseconds
pub const FRAME_TIME_MS: u64 = 16;
/// How long a received "is typing" signal stays visible without a refresh.
pub const TYPING_TTL: Duration = Duration::from_secs(3);
/// Minimum interval between outgoing "started typing" signals per conversation.
pub const TYPING_THROTTLE: Duration = Duration::from_millis(700);

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
/// Live media transport status for the TUI's active call (calls lane, US-044).
/// With the `tui-audio` feature the terminal also owns a real device adapter;
/// without it this mirrors the Rust media session's observable state only.
#[derive(Clone, Debug, Default)]
pub struct CallMediaStatus {
    /// Whether outgoing audio is muted in the Rust media session.
    pub muted: bool,
    /// Number of decoded peer frames received since the call was opened.
    pub frames_received: u64,
    /// First device error from the optional audio adapter, if any.
    pub audio_error: Option<String>,
}

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

/// Stable identity of a selected transcript row (US-006/008 parity).
///
/// Wire ids and DB ids survive history trims and row removals; the rendered
/// fallback also records which equal line was picked and is cleared whenever
/// its transcript mutates. A selection that can no longer be resolved is
/// treated as absent rather than silently targeting the row that happened to
/// inherit its old index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageSelection {
    /// A message with a stable wire id (normal incoming/outgoing rows).
    WireId(String),
    /// A sent row with no wire id, identified by its local DB id (US-005).
    DbId(i32),
    /// A live row with neither id: the exact rendered line, its ordinal among
    /// identical lines counted from the oldest loaded row, and that deque's
    /// length at selection time. Any history change invalidates it, so a trim
    /// cannot silently retarget an equal line.
    Rendered {
        text: String,
        occurrence: usize,
        loaded_len: usize,
    },
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
    /// Device captured when a location-track export prompt opens. The export
    /// always queries this exact sending device, never the user/contact or a
    /// different DM tab. `None` outside [`p2p_app::InputMode::ExportingLocationTrack`].
    pub location_export_peer_id: Option<String>,

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
    // Aggregated reactions by wire msg_id (US-008), refreshed on history loads
    // and on live `reaction` events from the swarm.
    pub reactions: HashMap<String, Vec<p2p_app::reactions::ReactionSummary>>,
    // Peers currently shown as typing in their DM pane (US-009), with the
    // expiry instant of the latest received signal.
    pub typing_peers: HashMap<String, Instant>,
    // Last outgoing "started typing" signal per peer, used to throttle key
    // repeats without suppressing an explicit stop.
    pub typing_sent_at: HashMap<String, Instant>,

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
    // Unread group messages per group while that chat is scrolled up
    pub group_unread_count: HashMap<String, usize>,
    // Selected group in the Groups list tab.
    pub group_selection: usize,

    // File shares (Shares tab)
    /// Shares known to this install, loaded from the canonical store at
    /// startup, on `share_*` events and on a manual `r` refresh.
    pub shares: Vec<p2p_app::shares::Share>,
    /// Grants by share id, re-read alongside the share list.
    pub share_grants: HashMap<String, Vec<p2p_app::shares::ShareGrant>>,
    /// Highlighted share in the Shares list.
    pub share_selection: usize,
    /// Highlighted grant in the selected share's detail pane.
    pub share_grant_selection: usize,
    /// Grant being added/revoked in the shared editor (`None` outside it).
    pub share_grant_prompt: Option<ShareGrantPrompt>,
    /// Latest share-lane event text, shown on the Shares tab.
    pub share_event_notice: Option<String>,
    /// Peer targeted by the Shares tab's network actions. Indexes the canonical
    /// sorted connected-peer list, never the `HashSet` iteration order.
    pub share_peer_selection: usize,

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
    // Unread DMs per peer while that conversation is scrolled up
    pub dm_unread_count: HashMap<String, usize>,
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

    /// Notifications built by Rust for incoming messages and waiting to be
    /// emitted as terminal escape sequences by the render loop (US-027/028).
    /// The queue keeps all TUI stdout writes in the render task, so a bell or
    /// OSC notification can never interleave with ratatui's frame drawing.
    pub pending_notifications: Vec<p2p_app::notifications::Notification>,

    /// Clipboard payloads queued by chat actions and emitted as OSC 52
    /// sequences by the render loop *after* the frame, so a copy can never
    /// interleave with ratatui's stdout writes (mirrors
    /// [`Self::pending_notifications`]).
    pub pending_clipboard: Vec<String>,

    /// Per-conversation reply drafts (US-006). Keyed by `chat`, `dm:<peer>` or
    /// `group:<id>` so a draft survives switching tabs but never leaks into a
    /// different conversation. `Ctrl+R` captures the targeted row; sending or
    /// `Esc` consumes/clears it.
    pub reply_drafts: HashMap<String, p2p_app::ReplyRef>,

    /// Explicitly selected transcript row per conversation (`Ctrl+Up`/`Ctrl+Down`).
    /// Absent means "newest", the historical default. The row is stored by a
    /// stable identity, not an index, so a trim or local deletion can never
    /// silently retarget a chat verb to a different message.
    pub message_selection: HashMap<String, MessageSelection>,

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

    /// A sent message being edited locally, pending text submission. The editor
    /// owns the shared `chat_input`, pre-filled with the original body; Enter
    /// applies the edit, Esc cancels. Device-only, like deletion (US-005).
    pub editing_message: Option<EditMessagePrompt>,

    /// A message being reacted to, pending an emoji token in the shared editor
    /// (US-008). Enter toggles the reaction locally and fans it out; Esc cancels.
    pub adding_reaction: Option<ReactionPrompt>,

    /// Live call session (calls lane). One session at a time, mirroring the
    /// single-slot device-binding prompt; `None` = no active/ringing call.
    /// Advanced here on call events; the interactive surface (accept/reject/
    /// hangup keys + prompt overlay + status line) lands in the TUI slice.
    pub call_session: Option<p2p_app::CallSession>,

    /// Media transport status for the active call (calls lane, US-044). With
    /// `tui-audio` the terminal also owns a device adapter; without it this
    /// mirrors the mic's mute state and the count of decoded peer frames.
    pub call_media: CallMediaStatus,

    /// Optional desktop audio adapter; a no-op handle when `tui-audio` is off.
    pub call_audio: TuiAudioHandle,

    // Settings tab (read-only node/network diagnostics)
    pub db_url: String,
    pub platform: String,

    /// Persisted bootstrap/relay multiaddrs (Settings tab), loaded at startup
    /// and after every add/remove. The node reads this list once at start.
    pub network_peers: Vec<p2p_app::network_config::NetworkPeer>,
    /// Highlighted row in the Settings network-peers list (`b`/`r` add,
    /// `x` removes the highlighted entry).
    pub network_peer_selection: usize,
    /// Kind captured by the active add prompt
    /// ([`p2p_app::InputMode::AddingNetworkPeer`]); `None` otherwise.
    pub network_peer_kind: Option<p2p_app::network_config::NetworkPeerKind>,

    /// Canonical palette snapshotted from Rust settings (US-025/US-026), so
    /// every renderer uses the same dark/light + accent choice as Flutter.
    pub palette: p2p_app::theme::Palette,

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

/// The sent message whose body the shared editor is replacing (US-005).
#[derive(Clone, Debug)]
pub struct EditMessagePrompt {
    pub message_id: i32,
    pub is_group: bool,
    pub tab_content: p2p_app::tui_tabs::TabContent,
}

/// The message being reacted to in the shared emoji editor (US-008). Only the
/// wire `msg_id` is needed: `reactions` is keyed by it across all chat kinds.
#[derive(Clone, Debug)]
pub struct ReactionPrompt {
    pub msg_id: String,
}

/// What the share-grant editor is doing. The mode belongs to the add action
/// only; a revoke prompt carries the user id it will remove so the typed text
/// is only a confirmation, never the authority for the target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShareGrantAction {
    /// Add or upgrade one user's access with the selected canonical mode.
    Add { mode: p2p_app::shares::ShareMode },
    /// Revoke an existing grant for this verified user id.
    Revoke { user_id: String },
}

/// The share whose grant the shared editor is changing (Flutter parity).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShareGrantPrompt {
    pub share_id: String,
    pub action: ShareGrantAction,
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
    /// True while a modal prompt owns the shared input. See
    /// [`p2p_app::InputMode::is_modal_editor`] for why Contacts search is not
    /// included.
    #[must_use]
    pub const fn is_modal_editor(&self) -> bool {
        self.input_mode.is_modal_editor()
    }

    /// True while a confirmation prompt or a live call covers the screen.
    ///
    /// These surfaces render above the ordinary popup and own both keyboard and
    /// mouse. `AppState::popup` and modal editors are deliberately excluded:
    /// a click still dismisses the visible popup, while an editor with no popup
    /// swallows navigation without being a full-screen confirmation.
    #[must_use]
    pub fn has_prompt_or_call_overlay(&self) -> bool {
        self.device_binding_prompt.is_some()
            || self.group_leave_prompt.is_some()
            || self.delete_message_prompt.is_some()
            || self
                .call_session
                .as_ref()
                .is_some_and(p2p_app::CallSession::is_active)
    }

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

    /// True while the shared input captures a device location-track filter.
    #[must_use]
    pub const fn is_exporting_location_track(&self) -> bool {
        matches!(self.input_mode, p2p_app::InputMode::ExportingLocationTrack)
    }

    /// Open the one-line `from;to` export prompt for a single DM device.
    pub fn begin_export_location_track(&mut self, peer_id: String) {
        self.input_mode = p2p_app::InputMode::ExportingLocationTrack;
        self.location_export_peer_id = Some(peer_id);
        self.chat_input = TextArea::default();
    }

    /// Close the export prompt without writing anything.
    pub fn cancel_export_location_track(&mut self) {
        if !self.is_exporting_location_track() {
            return;
        }
        self.input_mode = p2p_app::InputMode::Chat;
        self.location_export_peer_id = None;
        self.chat_input = TextArea::default();
    }

    pub fn cancel_nickname_edit(&mut self) {
        if !self.is_editing_nickname() {
            return;
        }
        self.input_mode = p2p_app::InputMode::Chat;
        self.editing_nickname_peer = None;
        self.chat_input = TextArea::default();
    }

    /// Finalize the shared single-line input when focus leaves a tab.
    ///
    /// Contacts search is an inline filter rather than a modal editor, so Tab,
    /// mouse clicks, and Peer Info shortcuts can move focus while it is open.
    /// Keep the applied query and clear the shared box instead of letting its
    /// text follow the user to the destination tab.
    pub fn finalize_tab_input(&mut self) {
        self.cancel_nickname_edit();
        self.cancel_share_grant();
        self.cancel_create_share();
        self.cancel_export_location_track();
        self.end_contact_search(true);
    }

    /// Move focus to `tab_index`, finalizing inline input first. Every keyboard
    /// and mouse tab transition goes through here so the shared editor never
    /// follows a Contacts query onto another tab.
    pub fn switch_to_tab(&mut self, tab_index: usize) {
        self.finalize_tab_input();
        self.active_tab = tab_index;
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

    /// Highlight a share and reset the grant cursor when the share changes.
    /// A grant index is only meaningful for the share it came from, so carrying
    /// it across rows could make `x` revoke the wrong user after a wheel/click.
    pub fn select_share(&mut self, index: usize) {
        let index = index.min(self.shares.len().saturating_sub(1));
        if index != self.share_selection {
            self.share_grant_selection = 0;
        }
        self.share_selection = index;
    }

    /// Connected peers in the canonical order used by the Shares network pane.
    #[must_use]
    pub fn share_peer_ids(&self) -> Vec<String> {
        self.connected.sorted_connected_peer_ids()
    }

    /// Peer the Shares network actions target, if any peer is connected.
    #[must_use]
    pub fn selected_share_peer_id(&self) -> Option<String> {
        self.share_peer_ids()
            .get(self.share_peer_selection)
            .cloned()
    }

    /// Keep the network-peer cursor inside the current connected-peer list.
    pub fn clamp_share_peer_selection(&mut self) {
        let len = self.share_peer_ids().len();
        self.share_peer_selection = self.share_peer_selection.min(len.saturating_sub(1));
    }

    /// Move the Shares network-peer cursor with Left/Right.
    pub fn move_share_peer_selection(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        let last = self.share_peer_ids().len().saturating_sub(1);
        self.share_peer_selection = match key {
            KeyCode::Left => self.share_peer_selection.saturating_sub(1),
            KeyCode::Right => self.share_peer_selection.saturating_add(1).min(last),
            _ => self.share_peer_selection,
        };
    }

    /// Clamp the grant cursor after a store reload changes the selected share's
    /// grants (for example, after the highlighted grant is revoked).
    fn clamp_share_grant_selection(&mut self) {
        let grant_count = self
            .shares
            .get(self.share_selection)
            .and_then(|share| self.share_grants.get(&share.share_id))
            .map_or(0, Vec::len);
        self.share_grant_selection = self
            .share_grant_selection
            .min(grant_count.saturating_sub(1));
    }

    /// Re-read the shares table. A DB failure keeps the last good snapshot and
    /// is logged, so a broken DB never blanks the tab (fail-open, like the
    /// other best-effort UI reloads).
    pub fn reload_shares(&mut self) {
        match p2p_app::shares::list_shares() {
            Ok(shares) => self.shares = shares,
            Err(error) => p2p_app::p2plog_debug(format!("Failed to reload shares: {error}")),
        }
        match p2p_app::shares::list_all_grants() {
            Ok(grants) => self.share_grants = grants,
            Err(error) => p2p_app::p2plog_debug(format!("Failed to reload share grants: {error}")),
        }
        self.select_share(self.share_selection);
        self.clamp_share_grant_selection();
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

    /// True while the shared text input is capturing a reaction emoji (US-008).
    #[must_use]
    pub const fn is_adding_reaction(&self) -> bool {
        self.adding_reaction.is_some()
    }

    /// True while the shared text input is replacing a sent message's body.
    #[must_use]
    pub const fn is_editing_message(&self) -> bool {
        self.editing_message.is_some()
    }

    /// Open the shared editor pre-filled with a sent message's full body.
    pub fn begin_edit_message(
        &mut self,
        message_id: i32,
        is_group: bool,
        original: String,
        tab_content: p2p_app::tui_tabs::TabContent,
    ) {
        self.input_mode = p2p_app::InputMode::EditingMessage;
        self.editing_message = Some(EditMessagePrompt {
            message_id,
            is_group,
            tab_content,
        });
        self.chat_input = TextArea::new(vec![original]);
    }

    /// Cancel an in-progress local edit, leaving the stored body untouched.
    pub fn cancel_edit_message(&mut self) -> bool {
        let was_editing = self.editing_message.take().is_some();
        if was_editing {
            self.input_mode = p2p_app::InputMode::Chat;
            self.chat_input = TextArea::default();
        }
        was_editing
    }

    /// Open the shared editor for a reaction on the targeted message in `tab`.
    /// Returns `false` when the conversation has no stable wire id to react to.
    pub fn begin_reaction(&mut self, tab: &p2p_app::tui_tabs::TabContent) -> bool {
        let Some(msg_id) = self.target_message_id(tab) else {
            return false;
        };
        self.input_mode = p2p_app::InputMode::AddingReaction;
        self.adding_reaction = Some(ReactionPrompt { msg_id });
        self.chat_input = TextArea::default();
        true
    }

    /// Cancel an in-progress reaction, leaving the stored reactions untouched.
    pub fn cancel_reaction(&mut self) -> bool {
        let was_adding = self.adding_reaction.take().is_some();
        if was_adding {
            self.input_mode = p2p_app::InputMode::Chat;
            self.chat_input = TextArea::default();
        }
        was_adding
    }

    /// True while the shared input captures a private-share grant user id.
    #[must_use]
    pub const fn is_editing_share_grant(&self) -> bool {
        matches!(self.input_mode, p2p_app::InputMode::EditingShareGrant)
    }

    /// True while the grant editor is a revoke confirmation. Its prefilled id
    /// is display-only: the stored action owns the target of the DB write.
    #[must_use]
    pub const fn is_revoking_share_grant(&self) -> bool {
        match self.share_grant_prompt.as_ref() {
            Some(prompt) => matches!(prompt.action, ShareGrantAction::Revoke { .. }),
            None => false,
        }
    }

    /// Grants on the currently highlighted share, empty when none is selected.
    #[must_use]
    pub fn selected_share_grants(&self) -> &[p2p_app::shares::ShareGrant] {
        self.shares
            .get(self.share_selection)
            .and_then(|share| self.share_grants.get(&share.share_id))
            .map_or(&[], Vec::as_slice)
    }

    /// Open the shared editor to add a grant on `share_id`, defaulting to the
    /// same `readonly` mode Flutter's add dialog starts with.
    pub fn begin_add_share_grant(&mut self, share_id: String) {
        self.input_mode = p2p_app::InputMode::EditingShareGrant;
        self.share_grant_prompt = Some(ShareGrantPrompt {
            share_id,
            action: ShareGrantAction::Add {
                mode: p2p_app::shares::ShareMode::ReadOnly,
            },
        });
        self.chat_input = TextArea::default();
    }

    /// Open the shared editor to revoke `user_id` from `share_id`. The id is
    /// pre-filled as a read-only confirmation; `Enter` commits the stored
    /// target and `Esc` cancels without changing the grant.
    pub fn begin_revoke_share_grant(&mut self, share_id: String, user_id: String) {
        self.input_mode = p2p_app::InputMode::EditingShareGrant;
        self.share_grant_prompt = Some(ShareGrantPrompt {
            share_id,
            action: ShareGrantAction::Revoke {
                user_id: user_id.clone(),
            },
        });
        self.chat_input = TextArea::new(vec![user_id]);
    }

    /// Switch the add prompt between the two canonical grant modes. Revoke
    /// prompts have no mode and are left untouched.
    pub const fn toggle_share_grant_mode(&mut self) {
        let Some(prompt) = self.share_grant_prompt.as_mut() else {
            return;
        };
        if let ShareGrantAction::Add { mode } = &mut prompt.action {
            *mode = match *mode {
                p2p_app::shares::ShareMode::ReadOnly => p2p_app::shares::ShareMode::ReadWrite,
                p2p_app::shares::ShareMode::ReadWrite => p2p_app::shares::ShareMode::ReadOnly,
            };
        }
    }

    /// Cancel an in-progress grant edit, leaving the persisted grants untouched.
    pub fn cancel_share_grant(&mut self) -> bool {
        let was_editing = self.share_grant_prompt.take().is_some();
        if was_editing {
            self.input_mode = p2p_app::InputMode::Chat;
            self.chat_input = TextArea::default();
        }
        was_editing
    }

    /// True while the shared input captures a new share display name.
    #[must_use]
    pub const fn is_creating_share(&self) -> bool {
        matches!(self.input_mode, p2p_app::InputMode::CreatingShare)
    }

    /// Open the shared editor for a new private, read-only share. No local root
    /// is captured here: binding a real folder stays a native-only action.
    pub fn begin_create_share(&mut self) {
        self.input_mode = p2p_app::InputMode::CreatingShare;
        self.chat_input = TextArea::default();
    }

    /// Close the share-name prompt without writing anything.
    pub fn cancel_create_share(&mut self) -> bool {
        let was_editing = self.is_creating_share();
        if was_editing {
            self.input_mode = p2p_app::InputMode::Chat;
            self.chat_input = TextArea::default();
        }
        was_editing
    }

    /// Move the highlighted grant in the selected share's detail pane. Bare
    /// arrows keep owning the share list; the modifier targets the grant.
    pub fn move_share_grant_selection(&mut self, delta: isize) {
        let last = self.selected_share_grants().len().saturating_sub(1);
        let current = self.share_grant_selection.min(last);
        self.share_grant_selection = if delta < 0 {
            current.saturating_sub(1)
        } else {
            current.saturating_add(1).min(last)
        };
    }

    /// Wire-id deque aligned with the transcript of `tab`, when it has one.
    fn message_ids_for_tab(
        &self,
        tab: &p2p_app::tui_tabs::TabContent,
    ) -> Option<&std::collections::VecDeque<Option<String>>> {
        use p2p_app::tui_tabs::TabContent;
        match tab {
            TabContent::Chat => Some(&self.message_ids),
            TabContent::Direct(peer_id) => self.dm_message_ids.get(peer_id),
            TabContent::GroupChat(group_id) => self.group_message_ids.get(group_id),
            _ => None,
        }
    }

    /// Wire id of the targeted message with a stable id in `tab` (US-008).
    /// The cursor pins the row; without one the newest replyable row is used.
    #[must_use]
    pub fn target_message_id(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<String> {
        let ids = self.message_ids_for_tab(tab)?;
        let idx = self.replyable_index(ids, tab)?;
        ids.get(idx)?.as_deref().map(str::to_owned)
    }

    /// Local path of a downloaded attachment in `tab`'s conversation.
    ///
    /// `Ctrl+O` opens this (parity with Flutter's tappable attachment chip).
    /// Transcript lines hold only the rendered chip text (`[file] name (size)`),
    /// not the `file_id`, so the descriptor is resolved from the persisted row
    /// by wire id. With no cursor this scans backwards for the newest row that
    /// actually has an attachment; with a cursor the picked row must itself
    /// carry one, so `Ctrl+O` never opens an unrelated file. Rows without a
    /// stable id (e.g. meshed group messages) have no lookup key and are skipped.
    #[must_use]
    pub fn target_attachment_path(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<String> {
        fn path_for_id(id: &str) -> Option<String> {
            if id.trim().is_empty() {
                return None;
            }
            let json = p2p_app::messages::attachment_json_for_msg_id(id)
                .or_else(|| p2p_app::groups::attachment_json_for_msg_id(id))?;
            let att = p2p_app::files::attachment_from_json(&json)?;
            p2p_app::files::file_path(&att.file_id)
        }

        let ids = self.message_ids_for_tab(tab)?;
        if self.message_selection_index(tab).is_some() {
            let idx = self.target_index(tab)?;
            return path_for_id(ids.get(idx)?.as_deref()?);
        }
        ids.iter()
            .rev()
            .find_map(|id| id.as_deref().and_then(path_for_id))
    }

    /// The most recent outgoing message in the conversation shown by `tab`,
    /// as `(db_id, short_preview)`; `None` when there is nothing deletable.
    #[must_use]
    pub fn last_sent_message(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<(i32, String)> {
        self.last_sent_message_raw(tab)
            .map(|(id, text)| (id, short_preview(&text)))
    }

    /// The most recent outgoing message with its FULL body, used to pre-fill
    /// the local-edit editor (the delete prompt wants the short preview only).
    #[must_use]
    pub fn last_sent_message_raw(
        &self,
        tab: &p2p_app::tui_tabs::TabContent,
    ) -> Option<(i32, String)> {
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
        let body = p2p_app::reply::display_body(text);
        Some((id, p2p_app::fmt::without_edited_marker(&body).to_string()))
    }

    /// Outgoing row under an explicit cursor. Unlike [`Self::last_sent_message_raw`]
    /// this never falls back to a different row: the operator picked one, so
    /// edit/delete must not silently act on another message.
    #[must_use]
    pub fn selected_sent_message_raw(
        &self,
        tab: &p2p_app::tui_tabs::TabContent,
    ) -> Option<(i32, String)> {
        use p2p_app::tui_tabs::TabContent;

        let idx = self.message_selection_index(tab)?;
        let (row_id, text) = match tab {
            TabContent::Chat => (
                *self.message_db_ids.get(idx)?,
                self.messages.get(idx).map_or("", |m| m.text.as_str()),
            ),
            TabContent::Direct(peer_id) => (
                *self.dm_message_db_ids.get(peer_id)?.get(idx)?,
                self.dm_messages
                    .get(peer_id)
                    .and_then(|msgs| msgs.get(idx))
                    .map_or("", String::as_str),
            ),
            TabContent::GroupChat(group_id) => (
                *self.group_message_db_ids.get(group_id)?.get(idx)?,
                self.group_messages
                    .get(group_id)
                    .and_then(|msgs| msgs.get(idx))
                    .map_or("", String::as_str),
            ),
            _ => return None,
        };
        let id = row_id?;
        let body = p2p_app::reply::display_body(text);
        Some((id, p2p_app::fmt::without_edited_marker(&body).to_string()))
    }

    /// Sent row the local verbs act on: the cursor's row when armed, otherwise
    /// the newest sent row (the historical default).
    #[must_use]
    pub fn target_sent_message(
        &self,
        tab: &p2p_app::tui_tabs::TabContent,
    ) -> Option<(i32, String)> {
        if self.message_selection_index(tab).is_some() {
            self.selected_sent_message_raw(tab)
                .map(|(id, text)| (id, short_preview(&text)))
        } else {
            self.last_sent_message(tab)
        }
    }

    /// Full-body counterpart of [`Self::target_sent_message`], used to pre-fill
    /// the local-edit editor.
    #[must_use]
    pub fn target_sent_message_raw(
        &self,
        tab: &p2p_app::tui_tabs::TabContent,
    ) -> Option<(i32, String)> {
        if self.message_selection_index(tab).is_some() {
            self.selected_sent_message_raw(tab)
        } else {
            self.last_sent_message_raw(tab)
        }
    }

    /// Notification focus key of the active tab (`broadcast`, `dm:<peer>`,
    /// `group:<id>`), or `None` for tabs without a chat conversation. The
    /// mapping lives with the notification payload so both sides use the same
    /// rule and do not suppress a message that is not on screen (US-028).
    #[must_use]
    pub fn active_conversation_key(&self) -> Option<String> {
        p2p_app::notifications::conversation_key_for_tab(
            &self.dynamic_tabs.tab_index_to_content(self.active_tab),
        )
    }

    /// Key shared by a conversation's reply draft and message cursor (`chat`,
    /// `dm:<peer>` or `group:<id>`). `None` for tabs without a message
    /// composer, so neither piece of state can leak into another conversation.
    #[must_use]
    pub fn conversation_key(tab: &p2p_app::tui_tabs::TabContent) -> Option<String> {
        use p2p_app::tui_tabs::TabContent;
        match tab {
            TabContent::Chat => Some("chat".to_string()),
            TabContent::Direct(peer_id) => Some(format!("dm:{peer_id}")),
            TabContent::GroupChat(group_id) => Some(format!("group:{group_id}")),
            _ => None,
        }
    }

    /// Drop a rendered-line anchor after its transcript changes.
    ///
    /// Wire and DB anchors keep resolving by identity. A rendered anchor cannot
    /// distinguish an append from a trim once the history cap is reached (the
    /// length stays equal), so any mutation clears it rather than risking a
    /// wrong target. Call this once after each aligned-queue append, reload,
    /// deletion or in-place line replacement.
    pub fn invalidate_rendered_selection(&mut self, tab: &p2p_app::tui_tabs::TabContent) {
        let Some(key) = Self::conversation_key(tab) else {
            return;
        };
        if matches!(
            self.message_selection.get(&key),
            Some(MessageSelection::Rendered { .. })
        ) {
            self.message_selection.remove(&key);
        }
    }

    /// Number of rendered rows in a conversation's transcript.
    fn transcript_len(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<usize> {
        use p2p_app::tui_tabs::TabContent;
        match tab {
            TabContent::Chat => Some(self.messages.len()),
            TabContent::Direct(peer_id) => self.dm_messages.get(peer_id).map(VecDeque::len),
            TabContent::GroupChat(group_id) => self.group_messages.get(group_id).map(VecDeque::len),
            _ => None,
        }
    }

    /// Rendered line of one transcript row.
    fn transcript_line<'a>(
        &'a self,
        tab: &p2p_app::tui_tabs::TabContent,
        index: usize,
    ) -> Option<&'a str> {
        use p2p_app::tui_tabs::TabContent;
        match tab {
            TabContent::Chat => self.messages.get(index).map(|m| m.text.as_str()),
            TabContent::Direct(peer_id) => self
                .dm_messages
                .get(peer_id)?
                .get(index)
                .map(String::as_str),
            TabContent::GroupChat(group_id) => self
                .group_messages
                .get(group_id)?
                .get(index)
                .map(String::as_str),
            _ => None,
        }
    }

    /// DB-row-id deque aligned with `tab`, when it has one.
    fn message_db_ids_for_tab(
        &self,
        tab: &p2p_app::tui_tabs::TabContent,
    ) -> Option<&std::collections::VecDeque<Option<i32>>> {
        use p2p_app::tui_tabs::TabContent;
        match tab {
            TabContent::Chat => Some(&self.message_db_ids),
            TabContent::Direct(peer_id) => self.dm_message_db_ids.get(peer_id),
            TabContent::GroupChat(group_id) => self.group_message_db_ids.get(group_id),
            _ => None,
        }
    }

    /// Stable identity of transcript row `index`, preferring the wire id, then
    /// the DB id, and finally the rendered line with its equal-line ordinal.
    fn selection_anchor(
        &self,
        tab: &p2p_app::tui_tabs::TabContent,
        index: usize,
    ) -> Option<MessageSelection> {
        if let Some(id) = self
            .message_ids_for_tab(tab)
            .and_then(|ids| ids.get(index))
            .and_then(|slot| slot.as_deref())
            .filter(|id| !id.trim().is_empty())
        {
            return Some(MessageSelection::WireId(id.to_string()));
        }
        if let Some(id) = self
            .message_db_ids_for_tab(tab)
            .and_then(|ids| ids.get(index))
            .copied()
            .flatten()
        {
            return Some(MessageSelection::DbId(id));
        }
        let line = self.transcript_line(tab, index)?;
        let occurrence = (0..index)
            .filter(|previous| self.transcript_line(tab, *previous) == Some(line))
            .count();
        Some(MessageSelection::Rendered {
            text: line.to_string(),
            occurrence,
            loaded_len: self.transcript_len(tab)?,
        })
    }

    /// Find the current row by its stable identity. A missing identity means
    /// the row was trimmed or deleted locally; callers then fall back to the
    /// newest row instead of acting on whatever inherited its old index.
    fn resolve_message_selection(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<usize> {
        let key = Self::conversation_key(tab)?;
        let anchor = self.message_selection.get(&key)?;
        let count = self.transcript_len(tab)?;
        match anchor {
            MessageSelection::WireId(id) => self
                .message_ids_for_tab(tab)?
                .iter()
                .position(|slot| slot.as_deref() == Some(id.as_str()))
                .filter(|index| *index < count),
            MessageSelection::DbId(db_id) => self
                .message_db_ids_for_tab(tab)?
                .iter()
                .position(|slot| *slot == Some(*db_id))
                .filter(|index| *index < count),
            MessageSelection::Rendered {
                text,
                occurrence,
                loaded_len,
            } => {
                if *loaded_len != count {
                    return None;
                }
                let mut seen = 0;
                for index in 0..count {
                    if self.transcript_line(tab, index) == Some(text.as_str()) {
                        if seen == *occurrence {
                            return Some(index);
                        }
                        seen = seen.saturating_add(1);
                    }
                }
                None
            }
        }
    }

    /// Row the chat verbs act on: the resolved explicit selection, otherwise
    /// the newest row. Every verb uses this so a reply, reaction, copy, open,
    /// edit or delete can never disagree about which message it targets.
    fn target_index(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<usize> {
        let count = self.transcript_len(tab)?;
        (count > 0).then(|| {
            self.resolve_message_selection(tab)
                .unwrap_or_else(|| count.saturating_sub(1))
        })
    }

    /// Arm the cursor on transcript row `index` (clamped to the conversation).
    /// Returns false for tabs without a transcript, or an empty one.
    pub fn select_message(&mut self, tab: &p2p_app::tui_tabs::TabContent, index: usize) -> bool {
        let Some(key) = Self::conversation_key(tab) else {
            return false;
        };
        let Some(count) = self.transcript_len(tab).filter(|count| *count > 0) else {
            return false;
        };
        let last = count.saturating_sub(1);
        let Some(anchor) = self.selection_anchor(tab, index.min(last)) else {
            return false;
        };
        self.message_selection.insert(key, anchor);
        true
    }

    /// Move the cursor by `delta` rows, starting from the newest when unset.
    /// Both ends clamp: wrapping would jump from the oldest row to an unrelated
    /// newest one on a single `Ctrl+Up` at the top of a transcript.
    pub fn move_message_selection(
        &mut self,
        tab: &p2p_app::tui_tabs::TabContent,
        delta: isize,
    ) -> bool {
        let Some(count) = self.transcript_len(tab).filter(|count| *count > 0) else {
            return false;
        };
        let current = self
            .resolve_message_selection(tab)
            .unwrap_or_else(|| count.saturating_sub(1));
        let step = delta.unsigned_abs();
        let next = if delta < 0 {
            current.saturating_sub(step)
        } else {
            current.saturating_add(step)
        }
        .min(count.saturating_sub(1));
        self.select_message(tab, next)
    }

    /// Drop the cursor for `tab` so verbs return to the newest row.
    pub fn clear_message_selection(&mut self, tab: &p2p_app::tui_tabs::TabContent) -> bool {
        let Some(key) = Self::conversation_key(tab) else {
            return false;
        };
        self.message_selection.remove(&key).is_some()
    }

    /// Resolved explicit cursor for the active conversation, used to highlight
    /// its row. Stale anchors resolve to `None` (never to a reused index).
    #[must_use]
    pub fn message_selection_index(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<usize> {
        self.resolve_message_selection(tab)
    }

    /// Rendered text of the targeted message in a chat tab, if any.
    ///
    /// This is the exact transcript line (time, sender tag, body) the user
    /// sees, so `Ctrl+C` copies the same row that `Ctrl+R`/`Ctrl+Y` target.
    /// Returns an owned string so the caller can queue it while mutating state.
    #[must_use]
    pub fn target_message_line(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<String> {
        use p2p_app::tui_tabs::TabContent;
        let idx = self.target_index(tab)?;
        let line = match tab {
            TabContent::Chat => self.messages.get(idx).map(|m| m.text.as_str()),
            TabContent::Direct(peer_id) => self
                .dm_messages
                .get(peer_id)
                .and_then(|msgs| msgs.get(idx))
                .map(String::as_str),
            TabContent::GroupChat(group_id) => self
                .group_messages
                .get(group_id)
                .and_then(|msgs| msgs.get(idx))
                .map(String::as_str),
            _ => None,
        }?;
        Some(line.to_string())
    }

    /// Capture the targeted replyable message in `tab` as a reply draft.
    ///
    /// With no explicit cursor this is the newest message with a stable wire
    /// id (the historical behaviour). With a cursor the picked row must itself
    /// carry an id, so a reply can never silently quote a different message.
    /// Attached files render as their canonical chip text in the transcript.
    pub fn begin_reply(
        &mut self,
        tab: &p2p_app::tui_tabs::TabContent,
    ) -> Option<p2p_app::ReplyRef> {
        use p2p_app::tui_tabs::TabContent;

        let key = Self::conversation_key(tab)?;
        let (line, id) = match tab {
            TabContent::Chat => {
                let idx = self.replyable_index(&self.message_ids, tab)?;
                (
                    self.messages.get(idx)?.text.as_str(),
                    self.message_ids.get(idx)?.as_deref()?,
                )
            }
            TabContent::Direct(peer_id) => {
                let ids = self.dm_message_ids.get(peer_id)?;
                let idx = self.replyable_index(ids, tab)?;
                (
                    self.dm_messages.get(peer_id)?.get(idx)?.as_str(),
                    ids.get(idx)?.as_deref()?,
                )
            }
            TabContent::GroupChat(group_id) => {
                let ids = self.group_message_ids.get(group_id)?;
                let idx = self.replyable_index(ids, tab)?;
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

    /// Index whose wire id is used by reply/reaction/open. An explicit cursor
    /// pins the row; without one we keep scanning backwards for the newest row
    /// with a stable id (meshed group rows can lack one).
    fn replyable_index(
        &self,
        ids: &std::collections::VecDeque<Option<String>>,
        tab: &p2p_app::tui_tabs::TabContent,
    ) -> Option<usize> {
        if self.message_selection_index(tab).is_some() {
            let idx = self.target_index(tab)?;
            return ids
                .get(idx)
                .and_then(|id| id.as_deref())
                .filter(|id| !id.trim().is_empty())
                .map(|_| idx);
        }
        ids.iter()
            .rposition(|id| id.as_deref().is_some_and(|id| !id.trim().is_empty()))
    }

    /// Draft currently attached to `tab`, if any.
    #[must_use]
    pub fn reply_draft(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<&p2p_app::ReplyRef> {
        let key = Self::conversation_key(tab)?;
        self.reply_drafts.get(key.as_str())
    }

    /// Consume the draft for `tab` and return it (used just before sending).
    pub fn take_reply(&mut self, tab: &p2p_app::tui_tabs::TabContent) -> Option<p2p_app::ReplyRef> {
        let key = Self::conversation_key(tab)?;
        self.reply_drafts.remove(key.as_str())
    }

    /// Clear the draft for `tab`; true when one was armed.
    pub fn clear_reply(&mut self, tab: &p2p_app::tui_tabs::TabContent) -> bool {
        let Some(key) = Self::conversation_key(tab) else {
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
                self.invalidate_rendered_selection(&TabContent::Chat);
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
                self.invalidate_rendered_selection(&TabContent::Direct(peer_id.clone()));
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
                self.invalidate_rendered_selection(&TabContent::GroupChat(group_id.clone()));
                true
            }
            _ => false,
        }
    }

    /// Refresh the prompted row's in-memory line after a successful local edit.
    /// The replacement is re-rendered from the DB row (text + attachment chip +
    /// marker), so it matches the next reload exactly. Returns false (leaving
    /// state untouched) if the row is no longer loaded.
    pub fn replace_edited_message(&mut self, prompt: &EditMessagePrompt, content: &str) -> bool {
        use p2p_app::tui_tabs::TabContent;

        let rendered = if prompt.is_group {
            p2p_app::groups::group_message_display_body(prompt.message_id)
        } else {
            p2p_app::messages::message_display_body(prompt.message_id)
        }
        .ok()
        .flatten()
        .unwrap_or_else(|| p2p_app::fmt::with_edited_marker(content));
        let replace = |line: &str| p2p_app::reply::with_body(line, &rendered);
        match &prompt.tab_content {
            TabContent::Chat => {
                let Some(idx) = index_of_db_id(&self.message_db_ids, prompt.message_id) else {
                    return false;
                };
                if let Some(line) = self.messages.get_mut(idx) {
                    let old = line.text.clone();
                    line.text = replace(&old);
                }
                self.invalidate_rendered_selection(&TabContent::Chat);
                true
            }
            TabContent::Direct(peer_id) => {
                let Some(idx) = self
                    .dm_message_db_ids
                    .get(peer_id)
                    .and_then(|ids| index_of_db_id(ids, prompt.message_id))
                else {
                    return false;
                };
                if let Some(msgs) = self.dm_messages.get_mut(peer_id)
                    && let Some(line) = msgs.get_mut(idx)
                {
                    let old = line.clone();
                    *line = replace(&old);
                }
                self.invalidate_rendered_selection(&TabContent::Direct(peer_id.clone()));
                true
            }
            TabContent::GroupChat(group_id) => {
                let Some(idx) = self
                    .group_message_db_ids
                    .get(group_id)
                    .and_then(|ids| index_of_db_id(ids, prompt.message_id))
                else {
                    return false;
                };
                if let Some(msgs) = self.group_messages.get_mut(group_id)
                    && let Some(line) = msgs.get_mut(idx)
                {
                    let old = line.clone();
                    *line = replace(&old);
                }
                self.invalidate_rendered_selection(&TabContent::GroupChat(group_id.clone()));
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
        self.group_unread_count.remove(group_id);
        self.reply_drafts.remove(&format!("group:{group_id}"));
        self.message_selection.remove(&format!("group:{group_id}"));
        if let Some(closed) = self.dynamic_tabs.remove_group_tab(group_id) {
            let next_tab = match closed.cmp(&self.active_tab) {
                // Leaving from the group's own chat tab: back to the list.
                Ordering::Equal => p2p_app::tui_tabs::FIXED_TAB_COUNT.saturating_sub(1),
                Ordering::Less => self.active_tab.saturating_sub(1),
                Ordering::Greater => self.active_tab,
            };
            self.switch_to_tab(next_tab);
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
        // Capture the selection BEFORE rebuilding/re-sorting. Rebuilding changes
        // row order, so reading the id afterwards (the old index into the new
        // list) would select whichever contact now occupies that position.
        let selected_contact_id = self.selected_contact_id();
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
        self.apply_contact_search_with(selected_contact_id);
    }

    /// Re-derive [`Self::contacts`] from the unfiltered [`Self::contacts_all`]
    /// with the active search query, keeping the currently selected contact
    /// selected when it survives the filter (otherwise the selection falls back
    /// to the first row).
    pub fn apply_contact_search(&mut self) {
        let previous = self.selected_contact_id();
        self.apply_contact_search_with(previous);
    }

    /// Shared filter+restore step for [`Self::apply_contact_search`] and
    /// [`Self::resort_peers`]. `previous` is the contact id selected before the
    /// list changed; callers that re-sort must capture it before rebuilding.
    fn apply_contact_search_with(&mut self, previous: Option<String>) {
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

    /// Peer whose info a Peer Info shortcut should show on `tab`.
    ///
    /// Contacts uses the selected device, Direct its partner, Chat/GroupChat
    /// the newest row with a known sender (our own rows have none), Settings
    /// the first connected peer (else first bound device), and every other tab
    /// has no target. Keeping the rule here means the keyboard binding and the
    /// mouse-click handler agree on what "the sender" is.
    #[must_use]
    pub fn peer_info_target(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<String> {
        use p2p_app::tui_tabs::TabContent;

        match tab {
            TabContent::Contacts => self.selected_contact_peer_id(),
            TabContent::Direct(peer_id) => Some(peer_id.clone()),
            TabContent::Chat => self
                .messages
                .iter()
                .rev()
                .find_map(|message| message.sender_peer_id.clone()),
            TabContent::GroupChat(group_id) => self
                .group_message_peer_ids
                .get(group_id)
                .and_then(|ids| ids.iter().rev().find_map(Clone::clone)),
            TabContent::Settings => {
                let connected: HashSet<String> = self
                    .connected
                    .connected_peer_ids()
                    .map(str::to_owned)
                    .collect();
                p2p_app::tui_render::settings_peer_target(&connected, &self.own_user_devices)
            }
            _ => None,
        }
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
            reactions: HashMap::new(),
            typing_peers: HashMap::new(),
            typing_sent_at: HashMap::new(),
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
            group_unread_count: HashMap::new(),
            group_selection: 0,
            shares: Vec::new(),
            share_grants: HashMap::new(),
            share_selection: 0,
            share_grant_selection: 0,
            share_grant_prompt: None,
            share_event_notice: None,
            share_peer_selection: 0,
            input_mode: p2p_app::InputMode::Chat,
            location_export_peer_id: None,
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
            dm_unread_count: HashMap::new(),
            dm_broadcast_scroll_state: HashMap::new(),
            own_nickname,
            local_peer_id,
            local_nicknames,
            received_nicknames,
            self_nicknames_for_peers,
            topic_str,
            editing_nickname_peer: None,
            popup: None,
            pending_notifications: Vec::new(),
            pending_clipboard: Vec::new(),
            reply_drafts: HashMap::new(),
            message_selection: HashMap::new(),
            device_binding_prompt: None,
            group_leave_prompt: None,
            delete_message_prompt: None,
            editing_message: None,
            adding_reaction: None,
            call_session: None,
            call_media: CallMediaStatus::default(),
            call_audio: TuiAudioHandle::default(),
            db_url: String::new(),
            platform: String::new(),
            network_peers: Vec::new(),
            palette: p2p_app::theme::Palette::current(),
            network_peer_selection: 0,
            network_peer_kind: None,
            own_user_id: None,
            own_user_devices: Vec::new(),
        };
        state.reload_blocked_peers();
        state.reload_shares();
        state.rebuild_contacts();
        state.contacts_all.clone_from(&state.contacts);
        state
    }

    /// Seed the broadcast badge from the persisted watermark at startup. With
    /// unread rows the pane opens on the first unread message (as in Flutter)
    /// instead of auto-scrolling past them while claiming "N new".
    pub fn seed_broadcast_unread(&mut self, unread: usize) {
        if unread == 0 || self.messages.is_empty() {
            return;
        }
        self.chat_unread_count = unread;
        self.auto_scroll.chat = false;
        self.chat_scroll_offset = self.messages.len().saturating_sub(unread);
    }

    /// Record that the newest broadcast is on screen: clear the badge and
    /// advance the shared DB watermark so Flutter/web agree after a restart.
    ///
    /// Best-effort by design — a settings write failure must never block input
    /// or navigation; it only means the badge may reappear next launch.
    pub fn mark_broadcast_viewed(&mut self) {
        self.chat_unread_count = 0;
        if let Err(e) = p2p_app::messages::mark_broadcast_viewed() {
            p2p_app::p2plog_debug(format!("Failed to persist broadcast viewed marker: {e}"));
        }
    }

    /// Apply a received typing signal (US-009). Returns whether the visible
    /// state changed, so the caller can request a redraw.
    pub fn receive_typing(&mut self, peer_id: &str, is_typing: bool) -> bool {
        if is_typing {
            let deadline = Instant::now()
                .checked_add(TYPING_TTL)
                .unwrap_or_else(Instant::now);
            self.typing_peers.insert(peer_id.to_string(), deadline);
            true
        } else {
            self.typing_peers.remove(peer_id).is_some()
        }
    }

    /// Drop expired typing indicators and report whether anything changed.
    pub fn expire_typing(&mut self) -> bool {
        let now = Instant::now();
        let before = self.typing_peers.len();
        self.typing_peers.retain(|_, deadline| *deadline > now);
        self.typing_peers.len() != before
    }

    /// Throttle outgoing "started typing" signals; stops are always sent.
    pub fn note_typing_sent(&mut self, peer_id: &str, is_typing: bool) -> bool {
        if !is_typing {
            self.typing_sent_at.remove(peer_id);
            return true;
        }
        let now = Instant::now();
        if self
            .typing_sent_at
            .get(peer_id)
            .is_some_and(|last| last.elapsed() < TYPING_THROTTLE)
        {
            return false;
        }
        self.typing_sent_at.insert(peer_id.to_string(), now);
        true
    }

    /// Forget both sides of a conversation's transient typing state.
    pub fn clear_typing(&mut self, peer_id: &str) -> bool {
        let removed = self.typing_peers.remove(peer_id).is_some();
        self.typing_sent_at.remove(peer_id);
        removed
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
        let body = p2p_app::fmt::with_edited_marker_if(msg.edited_at.is_some(), &body);
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
