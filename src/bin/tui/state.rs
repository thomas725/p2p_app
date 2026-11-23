use super::call_audio::TuiAudioHandle;
use super::{DynamicTabs, TextArea};
use p2p_app::contacts::ContactDeviceInput;
use p2p_app::messages::PeerMessageStats;
use p2p_app::tui_helpers::PeerMessageMap;
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

    /// Open the shared editor for a reaction on the newest message in `tab`.
    /// Returns `false` when the conversation has no stable wire id to react to.
    pub fn begin_reaction(&mut self, tab: &p2p_app::tui_tabs::TabContent) -> bool {
        let Some(msg_id) = self.newest_message_id(tab) else {
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

    /// Wire id of the newest message with a stable id in `tab` (US-008).
    #[must_use]
    pub fn newest_message_id(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<String> {
        self.message_ids_for_tab(tab)?.iter().rev().find_map(|id| {
            id.as_deref()
                .filter(|id| !id.trim().is_empty())
                .map(str::to_owned)
        })
    }

    /// Local path of the newest downloaded attachment in `tab`'s conversation.
    ///
    /// `Ctrl+O` opens this (parity with Flutter's tappable attachment chip).
    /// Transcript lines hold only the rendered chip text (`[file] name (size)`),
    /// not the `file_id`, so the descriptor is resolved from the persisted row
    /// by wire id — newest first. Rows without a stable id (e.g. meshed group
    /// messages without a wire id) have no lookup key and are skipped, just as
    /// they are for replies. `None` means nothing downloadable was found.
    #[must_use]
    pub fn newest_attachment_path(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<String> {
        self.message_ids_for_tab(tab)?.iter().rev().find_map(|id| {
            let id = id.as_deref().filter(|id| !id.trim().is_empty())?;
            let json = p2p_app::messages::attachment_json_for_msg_id(id)
                .or_else(|| p2p_app::groups::attachment_json_for_msg_id(id))?;
            let att = p2p_app::files::attachment_from_json(&json)?;
            p2p_app::files::file_path(&att.file_id)
        })
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

    /// Notification focus key of the active tab (`broadcast`, `dm:<peer>`,
    /// `group:<id>`), or `None` for tabs without a chat conversation. Mirrors
    /// `notifications::conversation_key` so a message arriving in the
    /// conversation already on screen does not fire a notification (US-028).
    #[must_use]
    pub fn active_conversation_key(&self) -> Option<String> {
        use p2p_app::tui_tabs::TabContent;
        match self.dynamic_tabs.tab_index_to_content(self.active_tab) {
            TabContent::Chat => Some("broadcast".to_string()),
            TabContent::Direct(peer) => Some(format!("dm:{peer}")),
            TabContent::GroupChat(id) => Some(format!("group:{id}")),
            _ => None,
        }
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

    /// Rendered text of the newest message in a chat tab, if any.
    ///
    /// This is the exact transcript line (time, sender tag, body) the user
    /// sees, so `Ctrl+C` copies the same row that `Ctrl+R`/`Ctrl+Y` target.
    /// Returns an owned string so the caller can queue it while mutating state.
    #[must_use]
    pub fn newest_message_line(&self, tab: &p2p_app::tui_tabs::TabContent) -> Option<String> {
        use p2p_app::tui_tabs::TabContent;
        let line = match tab {
            TabContent::Chat => self.messages.back().map(|m| m.text.as_str()),
            TabContent::Direct(peer_id) => self
                .dm_messages
                .get(peer_id)
                .and_then(VecDeque::back)
                .map(String::as_str),
            TabContent::GroupChat(group_id) => self
                .group_messages
                .get(group_id)
                .and_then(VecDeque::back)
                .map(String::as_str),
            _ => None,
        }?;
        Some(line.to_string())
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
            network_size: String::new(),
            network_peers: Vec::new(),
            palette: p2p_app::theme::Palette::current(),
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
