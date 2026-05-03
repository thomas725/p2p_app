//! Tests for logging.rs module

#[test]
fn test_push_log() {
    p2p_app::logging::push_log("test log message");
}

#[test]
fn test_strip_ansi_codes() {
    let input = "\x1b[31m\x1b[0m red text";
    let result = p2p_app::logging::strip_ansi_codes(input);
    assert!(!result.contains('\x1b'));
}

#[test]
fn test_strip_ansi_codes_plain() {
    let input = "plain text";
    let result = p2p_app::logging::strip_ansi_codes(input);
    assert_eq!(result, "plain text");
}
