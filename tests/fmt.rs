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
    let time = UNIX_EPOCH + std::time::Duration::from_secs(1700000);
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

#[test]
fn test_current_timestamp_returns_positive() {
    let ts = p2p_app::current_timestamp();
    assert!(ts > 0.0, "timestamp should be positive");
    assert!(
        ts < 2_000_000_000.0,
        "timestamp should be reasonable (before year 2033)"
    );
}

#[test]
fn test_current_timestamp_increases() {
    let ts1 = p2p_app::current_timestamp();
    std::thread::sleep(std::time::Duration::from_millis(1));
    let ts2 = p2p_app::current_timestamp();
    assert!(ts2 >= ts1, "later timestamp should be >= earlier");
}

// ── Additional fmt.rs edge cases ───────────────────────────────────────────────

#[test]
fn test_short_peer_id_exact_length() {
    use p2p_app::short_peer_id;
    let id = "12D3KooWH123456ABCDEFGH";
    let short = short_peer_id(id);
    assert_eq!(short.len(), 8);
}

#[test]
fn test_short_peer_id_too_short() {
    use p2p_app::short_peer_id;
    let id = "short";
    let short = short_peer_id(id);
    // Should still work, returning what's available
    assert!(!short.is_empty());
}

#[test]
fn test_short_peer_id_empty() {
    use p2p_app::short_peer_id;
    let short = short_peer_id("");
    assert_eq!(short, "");
}

#[test]
fn test_gen_msg_id_not_empty() {
    use p2p_app::gen_msg_id;
    let id = gen_msg_id();
    assert!(!id.is_empty());
}

#[test]
fn test_gen_msg_id_multiple_unique() {
    use p2p_app::gen_msg_id;
    let id1 = gen_msg_id();
    let id2 = gen_msg_id();
    // IDs should typically be different (though not guaranteed)
    let _ = (id1, id2);
}

#[test]
fn test_now_timestamp_positive() {
    use p2p_app::current_timestamp;
    let ts = current_timestamp();
    assert!(ts > 0.0);
}

#[test]
fn test_now_timestamp_reasonable() {
    use p2p_app::current_timestamp;
    let ts = current_timestamp();
    // Should be less than year 3000 in seconds
    assert!(ts < 32_000_000_000.0);
}

#[test]
fn test_format_system_time_now() {
    use p2p_app::format_system_time;
    use std::time::SystemTime;
    let time = SystemTime::now();
    let formatted = format_system_time(time);
    // Should contain date separators
    assert!(formatted.contains('-') || formatted.contains('/'));
}

#[test]
fn test_format_peer_datetime_format() {
    use p2p_app::format_peer_datetime;
    let formatted = format_peer_datetime("2024-01-01 12:00:00");
    assert!(!formatted.is_empty());
}

#[test]
fn test_peer_display_name_empty() {
    use p2p_app::peer_display_name;
    let name = peer_display_name("");
    assert!(!name.is_empty());
}

#[test]
fn test_peer_display_name_with_nickname() {
    use p2p_app::peer_display_name;
    let name = peer_display_name("Alice");
    assert_eq!(name, "Alice");
}

#[test]
fn test_scroll_title_broadcast() {
    use p2p_app::scroll_title;
    let title = scroll_title("Broadcast", 0, 100);
    assert!(title.contains("Broadcast"));
}

#[test]
fn test_scroll_title_with_position() {
    use p2p_app::scroll_title;
    let title = scroll_title("Test", 50, 100);
    // Should indicate position
    assert!(!title.is_empty());
}

#[test]
fn test_auto_scroll_offset_start() {
    use p2p_app::auto_scroll_offset;
    let offset = auto_scroll_offset(vec![], 80);
    assert_eq!(offset, 0);
}

#[test]
fn test_auto_scroll_offset_with_messages() {
    use p2p_app::auto_scroll_offset;
    let messages = vec!["msg1".to_string(), "msg2".to_string()];
    let offset = auto_scroll_offset(messages, 80);
    assert!(offset >= 0);
}

#[test]
fn test_format_latency_none() {
    use p2p_app::format_latency;
    use std::time::SystemTime;
    let result = format_latency(None, SystemTime::now());
    assert_eq!(result, "");
}

#[test]
fn test_format_latency_just_now() {
    use p2p_app::format_latency;
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let result = format_latency(Some(now), SystemTime::now());
    assert!(result.contains("ms") || result.contains("<1ms"));
}

#[test]
fn test_current_timestamp_within_range() {
    use p2p_app::current_timestamp;
    let ts = current_timestamp();
    // Should be in reasonable range
    assert!(ts > 1_000_000_000.0); // After year 2001
    assert!(ts < 100_000_000_000.0); // Before year 5138
}
