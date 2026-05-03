//! Tests for fmt.rs module

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_format_peer_datetime() {
    let dt = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();
    let result = p2p_app::fmt::format_peer_datetime(dt);
    assert!(result.contains("2024-01-15"));
    assert!(result.contains("10:30:00"));
}

#[test]
fn test_now_timestamp() {
    let result = p2p_app::fmt::now_timestamp();
    assert!(result.contains('-'));
    assert!(result.contains(':'));
}

#[test]
fn test_format_system_time() {
    let time = UNIX_EPOCH + std::time::Duration::from_millis(1700000000);
    let result = p2p_app::fmt::format_system_time(time);
    assert!(result.contains(':'));
    assert!(result.contains("000"));
}

#[test]
fn test_gen_msg_id() {
    let id1 = p2p_app::fmt::gen_msg_id();
    let id2 = p2p_app::fmt::gen_msg_id();
    assert!(!id1.is_empty());
    assert!(!id2.is_empty());
    assert_ne!(id1, id2);
}

#[test]
fn test_short_peer_id_empty() {
    let short = p2p_app::fmt::short_peer_id("");
    assert!(short.is_empty());
}

#[test]
fn test_short_peer_id_short() {
    let short = p2p_app::fmt::short_peer_id("ABC");
    assert_eq!(short, "ABC");
}

#[test]
fn test_peer_display_name_local_nickname() {
    let mut local = HashMap::new();
    local.insert("peer1".to_string(), "Alice".to_string());
    let received = HashMap::new();

    let name = p2p_app::fmt::peer_display_name("peer1", &local, &received);
    assert_eq!(name, "Alice");
}

#[test]
fn test_peer_display_name_received_nickname() {
    let local = HashMap::new();
    let mut received = HashMap::new();
    received.insert("peer1".to_string(), "Bob".to_string());

    let name = p2p_app::fmt::peer_display_name("peer1", &local, &received);
    assert_eq!(name, "Bob");
}

#[test]
fn test_peer_display_name_fallback() {
    let local = HashMap::new();
    let received = HashMap::new();

    // Use a longer peer ID to test fallback
    let name = p2p_app::fmt::peer_display_name("12D3KooWSkP1pEPy2", &local, &received);
    assert!(!name.is_empty());
}

#[test]
fn test_auto_scroll_offset() {
    assert_eq!(p2p_app::fmt::auto_scroll_offset(100, 20), 80);
    assert_eq!(p2p_app::fmt::auto_scroll_offset(10, 20), 0);
    assert_eq!(p2p_app::fmt::auto_scroll_offset(0, 10), 0);
}

#[test]
fn test_scroll_title() {
    let title = p2p_app::fmt::scroll_title("Messages", 10, 100);
    assert!(title.contains("Messages"));
    assert!(title.contains("(10/100)"));
}

#[test]
fn test_scroll_title_capped() {
    let title = p2p_app::fmt::scroll_title("Messages", 200, 100);
    assert!(title.contains("(100/100)"));
}

#[test]
fn test_format_latency_under_1ms() {
    let sent = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let now = SystemTime::now();
    let result = p2p_app::fmt::format_latency(Some(sent - 0.0001), now);
    assert_eq!(result, "<1ms");
}

#[test]
fn test_format_latency_under_1s() {
    let sent = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let now = SystemTime::now();
    let result = p2p_app::fmt::format_latency(Some(sent - 0.5), now);
    assert!(result.ends_with("ms"));
}

#[test]
fn test_format_latency_over_1s() {
    let sent = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let now = SystemTime::now();
    let result = p2p_app::fmt::format_latency(Some(sent - 2.0), now);
    assert!(result.ends_with('s'));
}

#[test]
fn test_format_latency_none() {
    let now = SystemTime::now();
    let result = p2p_app::fmt::format_latency(None, now);
    assert_eq!(result, "?");
}
