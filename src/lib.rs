//! # p2p_app
//!
//! Decentralized peer-to-peer chat on libp2p with TUI, CLI, and Dioxus desktop frontends.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │  frontends (bin/)                                    │
//! │  p2p_chat (CLI)  p2p_chat_tui (TUI)  p2p_chat_dioxus │
//! └────────────────────────┬─────────────────────────────┘
//!                          │ uses
//! ┌────────────────────────▼─────────────────────────────┐
//! │  p2p_app library                                     │
//! │  ┌──────────┬──────────┬──────────┬───────────────┐  │
//! │  │ behavior │  swarm   │ messages │  db / peers   │  │
//! │  │ network  │  handler │ nickname │  types / fmt  │  │
//! │  └──────────┴──────────┴──────────┴───────────────┘  │
//! │  ┌──────────────────────────────────────────┐        │
//! │  │  tui_render / tui_render_state / tui_tabs │        │
//! │  │  tui_helpers                             │        │
//! │  └──────────────────────────────────────────┘        │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! ## Feature flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `tui` | ratatui TUI (default) |
//! | `mdns` | mDNS peer discovery (default) |
//! | `tracing` | structured logging (default) |
//! | `quic` | QUIC transport (default) |
//! | `dioxus-desktop` | Dioxus desktop GUI |
//! | `test-utils` | test helpers (not in default) |
//!
//! ## Core modules
//!
//! - [`behavior`] - libp2p `NetworkBehaviour`, message protocols
//! - [`swarm_handler`] - event loop, translates libp2p events to `SwarmEvent`
//! - [`types`] - application-level event/command enums
//! - [`network`] - adaptive network size classification
//! - [`messages`] - message persistence and retrieval
//! - [`db`] - SQLite connection, identity, migrations
//! - [`peers`] - peer and session tracking
//! - [`nickname`] - nickname management
//! - [`fmt`] - formatting utilities
//! - [`logging`] - tracing-based logging with TUI callback

pub mod behavior;
pub mod db;
#[cfg(feature = "dioxus")]
pub mod dioxus_app;
#[cfg(feature = "dioxus")]
pub mod dioxus_styles;
#[cfg(feature = "dioxus")]
pub mod dioxus_swarm;
pub mod fmt;
pub mod logging;
pub mod messages;
pub mod network;
pub mod nickname;
pub mod peers;
pub mod swarm_handler;
pub mod tui_tabs;
pub mod types;

pub mod generated;

#[cfg(any(test, feature = "test-utils"))]
#[path = "../tests/shared/tui_test_state.rs"]
pub mod tui_test_state;

#[cfg(feature = "tui")]
pub mod tui_helpers;
#[cfg(feature = "tui")]
pub mod tui_render;
#[cfg(feature = "tui")]
pub mod tui_render_state;

pub use behavior::{
    AppBehaviour, BroadcastMessage, CHAT_TOPIC, ChatCodec, DM_PROTOCOL_NAME, DirectMessage,
    build_behaviour, build_swarm,
};
#[cfg(any(test, feature = "test-utils"))]
pub use db::reset_db_url_cache;
pub use db::{
    get_database_url, get_libp2p_identity, get_local_peer_id, init_database, release_db_lock,
    sqlite_connect,
};
pub use fmt::{
    auto_scroll_offset, current_timestamp, format_latency, format_peer_datetime,
    format_system_time, gen_msg_id, now_timestamp, peer_display_name, scroll_title, short_peer_id,
};
#[cfg(any(test, feature = "test-utils"))]
pub use logging::{clear_tui_logs, tracing_filter};
pub use logging::{
    get_tui_logs, init_logging, p2plog_debug, p2plog_error, p2plog_info, push_log,
    request_tui_redraw, set_tui_callback, set_tui_redraw_hook, strip_ansi_codes,
};
pub use messages::{
    MessageMeta, get_unsent_direct_messages, get_unsent_messages, load_direct_messages,
    load_messages, load_receipts, mark_message_sent, save_message, save_message_with_meta,
    save_receipt,
};
pub use network::{NetworkSize, get_network_size};
pub use nickname::{
    ensure_self_nickname, generate_self_nickname, get_peer_display_name, get_peer_local_nickname,
    get_peer_received_nickname, get_peer_self_nickname_for_peer, get_self_nickname,
    set_peer_local_nickname, set_peer_received_nickname, set_peer_self_nickname_for_peer,
    set_self_nickname,
};
pub use peers::{
    get_average_peer_count, get_recent_peer_count, load_known_peers, load_listen_ports, load_peers,
    save_listen_ports, save_peer, save_peer_session,
};
pub use swarm_handler::{build_broadcast_message, spawn_swarm_handler};
#[cfg(feature = "tui")]
pub use tui_helpers::{
    PAGE_SIZE, WHEEL_SCROLL_LINES, calculate_visible_range, disable_auto_scroll_to_max,
    handle_scroll_key_for_section, is_at_bottom, is_nickname_update, key_code_to_scroll_action,
    next_tab_index, parse_latency, relabel_dm_transcript, scroll_down_lines, scroll_up_lines,
    sort_peers_by_last_seen, truncate_message, upsert_peer_last_seen, validate_nickname,
};
#[cfg(feature = "tui")]
pub use tui_render::{
    render_chat_content, render_frame, render_peer_info, render_tab_content, render_tabs,
};
#[cfg(feature = "tui")]
pub use tui_render_state::{
    TuiRenderState, broadcast_receipt_prefix, calc_visible_strings, count_lines, dm_receipt_prefix,
    get_tab_content, row_to_visible_index,
};
#[cfg(feature = "tui")]
pub use tui_tabs::{DmTab, DynamicTabs, TabContent};
#[cfg(any(test, feature = "test-utils"))]
pub use tui_test_state::{NotificationTarget, TuiTestState};
pub use types::{DisplayMessage, MessageEvent, PeerRecord, SwarmCommand, SwarmEvent};

use diesel_migrations::{EmbeddedMigrations, embed_migrations};

/// Embedded SQLite migrations.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[cfg(test)]
#[path = "../tests/unit/unit_lib.rs"]
mod tests;
