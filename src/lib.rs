//! # `p2p_app` - Decentralized Peer-to-Peer Chat Application
//!
//! A fully decentralized peer-to-peer chat application built on top of libp2p.
//! This crate provides the core functionality for running a P2P chat node that can
//! communicate with other peers over a distributed network without requiring
//! central servers.
//!
//! ## Features
//!
//! - **Decentralized Messaging**: Send and receive messages directly peer-to-peer
//! - **Network Discovery**: Automatic peer discovery using mDNS
//! - **Message Persistence**: Store messages and metadata in `SQLite` database
//! - **TUI Interface**: Terminal user interface for interacting with the chat network
//! - **Direct Messaging**: Send private messages to specific peers
//! - **Nickname Management**: Set and manage nicknames for yourself and peers
//! - **Message Receipts**: Track message delivery and read status
//!
//! ## Core Modules
//!
//! - [`behavior`] - P2P swarm behavior and message handling
//! - [`network`] - Network size and connectivity information
//! - [`nickname`] - Nickname management for peers
//! - [`messages`] - Message storage and retrieval
//! - [`db`] - Database initialization and management
//! - [`peers`] - Peer information and session tracking
//! - [`swarm_handler`] - Swarm event handling
//! - [`types`] - Core type definitions
//! - [`logging`] - Logging functionality
//! - [`fmt`] - Formatting utilities

pub mod behavior;
pub mod db;
#[cfg(feature = "dioxus")]
pub mod dioxus_app;
pub mod fmt;
pub mod logging;
pub mod messages;
/// Network functionality for peer-to-peer communication
pub mod network;
/// Nickname management for chat users and peers
pub mod nickname;
pub mod peers;
/// Swarm handler for managing P2P network events
pub mod swarm_handler;
pub mod tui_tabs;
#[cfg(any(test, feature = "test-utils"))]
#[path = "../tests/tui_test_state.rs"]
pub mod tui_test_state;
/// Core type definitions used throughout the application
pub mod types;

/// Auto-generated database models and schema types
pub mod generated;

pub use behavior::{
    AppBehaviour, BroadcastMessage, CHAT_TOPIC, ChatCodec, DM_PROTOCOL_NAME, DirectMessage,
    build_behaviour,
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
pub use logging::clear_tui_logs;
#[cfg(any(test, feature = "test-utils"))]
pub use logging::tracing_filter;
pub use logging::{
    get_tui_logs, init_logging, p2plog_debug, p2plog_error, p2plog_info, p2plog_warn, push_log,
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
pub use swarm_handler::spawn_swarm_handler;
#[cfg(feature = "tui")]
pub use tui_tabs::{DmTab, DynamicTabs, TabContent, TabId};
#[cfg(feature = "tui")]
pub mod tui_helpers;
#[cfg(any(test, feature = "test-utils"))]
pub use tui_test_state::{NotificationTarget, TuiTestState};
#[cfg(feature = "tui")]
pub mod tui_render;
#[cfg(feature = "tui")]
pub mod tui_render_state;
#[cfg(feature = "tui")]
pub use tui_render::{
    render_chat_content, render_frame, render_peer_info, render_tab_content, render_tabs,
};
#[cfg(feature = "tui")]
pub use tui_render_state::{
    TuiRenderState, broadcast_receipt_prefix, calc_visible_strings, count_lines, dm_receipt_prefix,
    get_tab_content, row_to_visible_index,
};
pub use types::{SwarmCommand, SwarmEvent};

use diesel_migrations::{EmbeddedMigrations, embed_migrations};

/// Database migrations for `p2p_app` schema initialization and updates.
///
/// This constant contains all SQL migrations embedded at compile time.
/// It's used to initialize and upgrade the database schema when the application starts.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[cfg(test)]
#[path = "../tests/unit/unit_lib.rs"]
mod tests;
