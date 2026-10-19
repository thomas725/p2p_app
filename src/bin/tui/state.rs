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
    /// Whether curses-style mouse events are captured (Esc toggles).
    pub mouse_capture: bool,
    /// One-shot flag: display names have been resolved for every known peer the
    /// first time the Peers tab is opened (warms the library's name cache).
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
    // Broadcast receipts: msg_id -> (peer_id -> received_at epoch seconds).
    pub broadcast_receipts: HashMap<String, HashMap<String, f64>>,
    pub dm_messages: HashMap<String, VecDeque<String>>,
    // DM message IDs aligned with dm_messages[peer_id].
    pub dm_message_ids: HashMap<String, VecDeque<Option<String>>>,
    // DM receipts: msg_id -> (peer_id, received_at epoch seconds).
    pub dm_receipts: HashMap<String, (String, f64)>,

    // Peer Management
    pub peers: VecDeque<p2p_app::PeerRecord>,
    // Canonical Contacts-tab rows (from `contacts::build_contact_rows`).
    // Rebuilt whenever the device list changes (see `rebuild_contacts`); the
    // live `peers` deque stays the single device source, and grouped users
    // collapse here into one row per contact.
    pub contacts: Vec<p2p_app::contacts::ContactRow>,
    // Contact IDs expanded into their device sub-rows on the Contacts tab.
    pub expanded_contact_ids: HashSet<String>,
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
    pub editing_nickname_peer: Option<String>,

    // Ad-hoc UI popup (used for receipt timing details, etc.)
    pub popup: Option<String>,

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

    /// Live call session (calls lane). One session at a time, mirroring the
    /// single-slot device-binding prompt; `None` = no active/ringing call.
    /// Advanced here on call events; the interactive surface (accept/reject/
    /// hangup keys + prompt overlay + status line) lands in the TUI slice.
    pub call_session: Option<p2p_app::CallSession>,

    // Settings tab (read-only node/network diagnostics)
    pub db_url: String,
    pub platform: String,
    pub network_size: String,
    pub listen_addrs: Vec<String>,

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

impl AppState {
    /// True while the shared text input is capturing a nickname edit.
    pub const fn is_editing_nickname(&self) -> bool {
        matches!(self.input_mode, p2p_app::InputMode::EditingNickname)
    }

    /// True while the shared text input is capturing a group name.
    pub const fn is_creating_group(&self) -> bool {
        matches!(self.input_mode, p2p_app::InputMode::CreatingGroup)
    }

    pub fn cancel_nickname_edit(&mut self) {
        if !self.is_editing_nickname() {
            return;
        }
        self.input_mode = p2p_app::InputMode::Chat;
        self.editing_nickname_peer = None;
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
        self.group_message_peer_ids.remove(group_id);
        self.group_scroll_state.remove(group_id);
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
        self.broadcast_sent_to_peer = p2p_app::messages::get_all_peer_stats()
            .map(|stats| {
                stats
                    .into_iter()
                    .map(|(pid, s)| (pid, usize::try_from(s.broadcast_sent_to_peer).unwrap_or(0)))
                    .collect()
            })
            .unwrap_or_default();
        let previous = self.selected_contact_id();
        self.rebuild_contacts();
        self.contacts = p2p_app::contacts::sort_canonical_rows(
            &self.contacts,
            self.peer_sort_column,
            self.peer_sort_ascending,
        );
        self.peer_selection = previous
            .and_then(|contact_id| {
                self.contacts
                    .iter()
                    .position(|c| c.contact_id == contact_id)
            })
            .unwrap_or(0)
            .min(self.contacts.len().saturating_sub(1));
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
            .map(|contacts| {
                contacts
                    .into_iter()
                    .filter_map(|c| c.nickname_override.map(|n| (c.contact_user_id, n)))
                    .collect()
            })
            .unwrap_or_default();

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
            broadcast_receipts: data.broadcast_receipts,
            dm_messages: HashMap::new(),
            dm_message_ids: HashMap::new(),
            dm_receipts: data.dm_receipts,
            peers: data.peers,
            contacts: Vec::new(),
            expanded_contact_ids: HashSet::new(),
            broadcast_sent_to_peer: HashMap::new(),
            group_summaries: data.groups,
            group_messages: HashMap::new(),
            group_message_ids: HashMap::new(),
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
            device_binding_prompt: None,
            group_leave_prompt: None,
            call_session: None,
            db_url: String::new(),
            platform: String::new(),
            network_size: String::new(),
            listen_addrs: Vec::new(),
            own_user_id: None,
            own_user_devices: Vec::new(),
        };
        state.rebuild_contacts();
        state
    }
}

type FormattedMessages = (VecDeque<p2p_app::DisplayMessage>, VecDeque<Option<String>>);

/// Pure: formats DB messages into display-ready `(text, peer_id)` pairs.
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
        messages.push_back(p2p_app::DisplayMessage {
            text: format!("{ts} {sender} {body}"),
            sender_peer_id: msg.peer_id.clone(),
        });
        message_ids.push_back(msg.msg_id.clone());
    }
    (messages, message_ids)
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
            (VecDeque::new(), VecDeque::new())
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

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_state.rs"]
mod tests;
