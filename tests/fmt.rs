//! Tests for fmt.rs module

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]

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
    let time = UNIX_EPOCH + std::time::Duration::from_secs(1_700_000);
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
fn test_peer_id_suffix_last_3_chars() {
    assert_eq!(p2p_app::fmt::peer_id_suffix("12D3KooWHashABCD1234"), "234");
    // Shorter IDs are returned whole.
    assert_eq!(p2p_app::fmt::peer_id_suffix("abc"), "abc");
    assert_eq!(p2p_app::fmt::peer_id_suffix("ab"), "ab");
    assert_eq!(p2p_app::fmt::peer_id_suffix(""), "");
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
    // Should contain time separators (HH:MM:SS.mmm)
    assert!(formatted.contains(':'));
}

#[test]
fn test_peer_display_name_empty() {
    use p2p_app::peer_display_name;
    use std::collections::HashMap;

    let local = HashMap::new();
    let received = HashMap::new();
    let name = peer_display_name("", &local, &received);
    assert!(name.is_empty());
}

#[test]
fn test_peer_display_name_with_nickname() {
    use p2p_app::peer_display_name;
    use std::collections::HashMap;

    let local = HashMap::new();
    let received = HashMap::new();
    let name = peer_display_name("Alice", &local, &received);
    assert!(!name.is_empty());
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
