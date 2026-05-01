pub mod behavior;
pub mod db;
pub mod fmt;
pub mod logging;
pub mod logging_config;
pub mod messages;
pub mod network;
pub mod nickname;
pub mod peers;
pub mod swarm_handler;
pub mod tui_events;
pub mod tui_tabs;
pub mod tui_test_state;
pub mod types;

pub mod generated;

pub use behavior::{
    AppBehaviour, BroadcastMessage, CHAT_TOPIC, ChatCodec, DM_PROTOCOL_NAME, DirectMessage,
    build_behaviour,
};
pub use db::{
    get_database_url, get_libp2p_identity, get_local_peer_id, init_database, release_db_lock,
    sqlite_connect,
};
pub use fmt::{
    auto_scroll_offset, format_latency, format_peer_datetime, format_system_time, gen_msg_id,
    now_timestamp, peer_display_name, scroll_title, short_peer_id,
};
pub use logging::{
    get_tui_logs, init_logging, p2plog_debug, p2plog_error, p2plog_info, p2plog_warn, push_log,
    set_tui_callback, strip_ansi_codes,
};
pub use logging_config::tracing_filter;
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
pub use tui_test_state::{NotificationTarget, TuiTestState};
pub use types::{SwarmCommand, SwarmEvent};

use diesel_migrations::{EmbeddedMigrations, embed_migrations};
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_peer_datetime() -> Result<(), chrono::ParseError> {
        use chrono::NaiveDateTime;
        let dt = NaiveDateTime::parse_from_str("2024-01-01 12:00:00", "%Y-%m-%d %H:%M:%S")?;
        let formatted = format_peer_datetime(dt);
        assert!(formatted.contains("2024"));
        Ok(())
    }

    #[test]
    fn test_strip_ansi_codes() {
        let ansi_text = "\x1b[32mHello\x1b[0m";
        let clean = logging::strip_ansi_codes(ansi_text);
        assert_eq!(clean, "Hello");
    }

    #[test]
    fn test_now_timestamp_format() {
        let ts = now_timestamp();
        assert!(ts.contains('-'));
        assert!(ts.contains(':'));
    }

    #[test]
    fn test_network_size_from_peer_count() {
        assert_eq!(
            network::NetworkSize::from_peer_count(1.0),
            network::NetworkSize::Small
        );
        assert_eq!(
            network::NetworkSize::from_peer_count(5.0),
            network::NetworkSize::Medium
        );
        assert_eq!(
            network::NetworkSize::from_peer_count(20.0),
            network::NetworkSize::Large
        );
    }

    #[test]
    fn test_network_size_boundary_values() {
        assert_eq!(
            network::NetworkSize::from_peer_count(0.0),
            network::NetworkSize::Small
        );
        assert_eq!(
            network::NetworkSize::from_peer_count(3.0),
            network::NetworkSize::Small
        );
        assert_eq!(
            network::NetworkSize::from_peer_count(4.0),
            network::NetworkSize::Medium
        );
        assert_eq!(
            network::NetworkSize::from_peer_count(15.0),
            network::NetworkSize::Medium
        );
        assert_eq!(
            network::NetworkSize::from_peer_count(16.0),
            network::NetworkSize::Large
        );
    }

    #[test]
    fn test_network_size_display() {
        assert_eq!(format!("{:?}", network::NetworkSize::Small), "Small");
        assert_eq!(format!("{:?}", network::NetworkSize::Medium), "Medium");
        assert_eq!(format!("{:?}", network::NetworkSize::Large), "Large");
    }

    #[test]
    fn test_network_size_from_peer_count_method() {
        let small = network::NetworkSize::from_peer_count(3.0);
        let medium = network::NetworkSize::from_peer_count(10.0);
        let large = network::NetworkSize::from_peer_count(100.0);
        assert_eq!(small, network::NetworkSize::Small);
        assert_eq!(medium, network::NetworkSize::Medium);
        assert_eq!(large, network::NetworkSize::Large);
    }

    #[test]
    fn test_strip_ansi_codes_empty() {
        let clean = logging::strip_ansi_codes("");
        assert_eq!(clean, "");
    }

    #[test]
    fn test_strip_ansi_codes_no_ansi() {
        let clean = logging::strip_ansi_codes("plain text");
        assert_eq!(clean, "plain text");
    }

    #[test]
    fn test_strip_ansi_codes_multiple_codes() {
        let ansi = "\x1b[1m\x1b[32mBold Green\x1b[0m Normal";
        let clean = logging::strip_ansi_codes(ansi);
        assert_eq!(clean, "Bold Green Normal");
    }

    #[test]
    fn test_strip_ansi_codes_incomplete_sequence() {
        let ansi = "\x1b[32mGreen\x1b[0m incomplete";
        let clean = logging::strip_ansi_codes(ansi);
        assert!(clean.contains("Green"));
        assert!(clean.contains("incomplete"));
    }

    #[test]
    fn test_direct_message_serialization() {
        use serde_json;
        let dm = DirectMessage {
            content: "Hello".to_string(),
            timestamp: 1234567890,
            sent_at: Some(1234567890.5),
            nickname: None,
            msg_id: None,
            ack_for: None,
            received_at: None,
        };
        let json = serde_json::to_string(&dm).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("1234567890"));
    }

    #[test]
    fn test_broadcast_message_serialization() {
        use serde_json;
        let bm = BroadcastMessage {
            content: "World".to_string(),
            sent_at: Some(1234567890.5),
            nickname: None,
            msg_id: None,
        };
        let json = serde_json::to_string(&bm).unwrap();
        assert!(json.contains("World"));
    }

    #[test]
    fn test_network_size_equality() {
        assert_eq!(network::NetworkSize::Small, network::NetworkSize::Small);
        assert_eq!(network::NetworkSize::Medium, network::NetworkSize::Medium);
        assert_eq!(network::NetworkSize::Large, network::NetworkSize::Large);
        assert_ne!(network::NetworkSize::Small, network::NetworkSize::Medium);
    }

    #[test]
    fn test_network_size_copy() {
        let size = network::NetworkSize::Medium;
        let copy = size;
        assert_eq!(size, copy);
    }

    #[test]
    fn test_short_peer_id() {
        let long_id = "1234567890abcdef";
        let short = fmt::short_peer_id(long_id);
        assert_eq!(short, "90abcdef");
    }

    #[test]
    fn test_short_peer_id_short_input() {
        let short_id = "abc";
        let short = fmt::short_peer_id(short_id);
        assert_eq!(short, "abc");
    }

    #[test]
    fn test_auto_scroll_offset() {
        assert_eq!(fmt::auto_scroll_offset(10, 5), 5);
        assert_eq!(fmt::auto_scroll_offset(5, 10), 0);
    }

    #[test]
    fn test_scroll_title() {
        let title = fmt::scroll_title("Messages", 3, 10);
        assert!(title.contains("Messages"));
        assert!(title.contains("(3/10)"));
    }

    #[test]
    fn test_peer_display_name_local_nickname() {
        use std::collections::HashMap;
        let mut local = HashMap::new();
        local.insert("peer1".to_string(), "Alice".to_string());
        let received = HashMap::new();
        let name = fmt::peer_display_name("peer1", &local, &received);
        assert_eq!(name, "Alice");
    }

    #[test]
    fn test_peer_display_name_received_nickname() {
        use std::collections::HashMap;
        let local = HashMap::new();
        let mut received = HashMap::new();
        received.insert("peer1".to_string(), "Bob".to_string());
        let name = fmt::peer_display_name("peer1", &local, &received);
        assert_eq!(name, "Bob");
    }

    #[test]
    fn test_peer_display_name_fallback() {
        use std::collections::HashMap;
        let local = HashMap::new();
        let received = HashMap::new();
        let name = fmt::peer_display_name("peer1", &local, &received);
        assert_eq!(name, "peer1");
    }

    #[test]
    fn test_gen_msg_id_uniqueness() {
        let id1 = fmt::gen_msg_id();
        let id2 = fmt::gen_msg_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_gen_msg_id_format() {
        let id = fmt::gen_msg_id();
        assert!(!id.is_empty());
    }

    #[test]
    fn test_format_system_time() {
        use std::time::UNIX_EPOCH;
        let time = UNIX_EPOCH + std::time::Duration::from_millis(1234567);
        let formatted = fmt::format_system_time(time);
        assert!(formatted.contains(':'));
    }

    #[test]
    fn test_format_latency_sub_millisecond() {
        use std::time::UNIX_EPOCH;
        let _sent = UNIX_EPOCH;
        let received = UNIX_EPOCH + std::time::Duration::from_millis(0);
        let latency = fmt::format_latency(Some(0.0), received);
        assert_eq!(latency, "<1ms");
    }

    #[test]
    fn test_format_latency_milliseconds() {
        let sent = 0.0;
        let now = 0.5;
        let received = std::time::UNIX_EPOCH + std::time::Duration::from_secs_f64(now);
        let latency = fmt::format_latency(Some(sent), received);
        assert!(latency.contains("ms"));
    }

    #[test]
    fn test_format_latency_seconds() {
        let sent = 0.0;
        let now = 2.0;
        let received = std::time::UNIX_EPOCH + std::time::Duration::from_secs_f64(now);
        let latency = fmt::format_latency(Some(sent), received);
        assert!(latency.contains('s'));
    }

    #[test]
    fn test_format_latency_none() {
        use std::time::SystemTime;
        let latency = fmt::format_latency(None, SystemTime::now());
        assert_eq!(latency, "?");
    }
}
