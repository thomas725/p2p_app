pub mod logging;
pub mod models_insertable;
pub mod models_queryable;
pub mod network;
pub mod nickname;
pub mod schema;
pub mod types;
pub mod swarm_handler;
pub mod fmt;
pub mod behavior;
pub mod db;
pub mod messages;
pub mod peers;
pub mod logging_config;
#[cfg(feature = "tui")]
pub mod tui_tabs;
#[cfg(feature = "tui")]
pub mod tui_test_state;
#[cfg(feature = "tui")]
pub mod tui_events;

pub use logging::init_logging;
pub use network::{NetworkSize, get_network_size};
pub use nickname::{
    ensure_self_nickname, generate_self_nickname, get_peer_display_name, get_peer_local_nickname,
    get_peer_received_nickname, get_self_nickname, set_peer_local_nickname,
    set_peer_received_nickname, set_self_nickname,
};
pub use types::{SwarmCommand, SwarmEvent};
pub use swarm_handler::spawn_swarm_handler;
pub use fmt::{
    auto_scroll_offset, format_latency, format_peer_datetime, format_system_time, now_timestamp,
    peer_display_name, scroll_title, short_peer_id,
};
pub use behavior::{
    build_behaviour, AppBehaviour, BroadcastMessage, ChatCodec, DirectMessage, CHAT_TOPIC,
    DM_PROTOCOL_NAME,
};
pub use db::{get_database_url, get_libp2p_identity, sqlite_connect};
pub use messages::{
    get_unsent_direct_messages, get_unsent_messages, load_direct_messages, load_messages,
    mark_message_sent, save_message,
};
pub use peers::{
    get_average_peer_count, get_recent_peer_count, load_listen_ports, load_peers, save_listen_ports,
    save_peer, save_peer_session,
};
pub use logging_config::tracing_filter;
#[cfg(feature = "tui")]
pub use tui_tabs::{DynamicTabs, TabContent, TabId, DmTab};
#[cfg(feature = "tui")]
pub use tui_test_state::{TuiTestState, NotificationTarget};

use diesel_migrations::{EmbeddedMigrations, embed_migrations};
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[cfg(feature = "tui")]
pub fn log_debug(
    logs: &std::sync::Mutex<std::collections::VecDeque<String>>,
    message: impl Into<String>,
) {
    let ts = format_system_time(std::time::SystemTime::now());
    let formatted = format!("[{}] {}", ts, message.into());
    if let Ok(mut l) = logs.lock() {
        l.push_back(formatted);
    }
}

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
}



