use crate::reply::{
    ReplyRef, decode, display_body, encode, from_display_line, preview_line, quoted_header,
    split_display_line, with_body,
};

#[test]
fn preview_line_collapses_and_bounds_text() {
    assert_eq!(preview_line("  hello\n world  "), "hello world");
    let long = "x ".repeat(60);
    let preview = preview_line(&long);
    assert_eq!(preview.chars().count(), 81);
    assert!(preview.ends_with('…'));
}

#[test]
fn snapshot_round_trips_through_storage_json() {
    let reply = ReplyRef::new("msg-1".into(), "Alice (abc)".into(), "hello there");
    let raw = encode(&reply).expect("serialize reply");
    assert_eq!(decode(&raw), Some(reply));
}

#[test]
fn quoted_header_uses_stored_snapshot() {
    let reply = ReplyRef::new("msg-1".into(), "Alice".into(), "hello");
    assert_eq!(quoted_header(&reply), "↳ Alice: hello");
}

#[test]
fn tui_display_line_capture_keeps_sender_and_body() {
    let reply = from_display_line("12:34 [Alice (abc)] hello there", "msg-1")
        .expect("capture reply from transcript line");
    assert_eq!(
        reply,
        ReplyRef::new("msg-1".into(), "Alice (abc)".into(), "hello there")
    );
}

#[test]
fn tui_display_line_capture_skips_an_existing_quote_header() {
    let line = "↳ Bob: earlier text\n12:34 [Alice (abc)] hello there";
    let reply = from_display_line(line, "msg-2").expect("capture reply from quoted line");
    assert_eq!(reply.sender, "Alice (abc)");
    assert_eq!(reply.preview, "hello there");
}

#[test]
fn without_a_wire_id_there_is_no_reply_capture() {
    assert_eq!(from_display_line("12:34 [Alice] hello", ""), None);
}

#[test]
fn display_line_body_can_be_replaced_without_losing_the_prefix() {
    let line = "12:34 [Alice (abc)] original body";
    assert_eq!(split_display_line(line), ("Alice (abc)", "original body"));
    assert_eq!(display_body(line), "original body");
    assert_eq!(
        with_body(line, "replacement (edited)"),
        "12:34 [Alice (abc)] replacement (edited)"
    );
}

#[test]
fn replacing_a_quoted_line_keeps_the_quote_header() {
    let line = "12:34 [5ms] [Alice] ↳ Bob: earlier text\n12:34 [Alice] original";
    assert_eq!(
        with_body(line, "replacement"),
        "12:34 [5ms] [Alice] ↳ Bob: earlier text\n12:34 [Alice] replacement"
    );
}

#[test]
fn multiline_body_keeps_its_sender_and_every_line() {
    // A message composed across several lines stores its sender tag on the
    // first row only; the old last-line parser captured an empty sender and
    // just the final line. The body must survive in full.
    let line = "12:34 [Alice (abc)] line one\nline two\nline three";
    assert_eq!(
        split_display_line(line),
        ("Alice (abc)", "line one\nline two\nline three")
    );
    assert_eq!(display_body(line), "line one\nline two\nline three");

    let reply = from_display_line(line, "msg-multi").expect("capture multiline reply");
    assert_eq!(reply.sender, "Alice (abc)");
    assert_eq!(reply.preview, "line one line two line three");
}

#[test]
fn multiline_reply_capture_skips_the_quote_header() {
    let line = "↳ Bob: earlier text\n12:34 [Alice] line one\nline two";
    let reply = from_display_line(line, "msg-multi").expect("capture quoted multiline reply");
    assert_eq!(reply.sender, "Alice");
    assert_eq!(reply.preview, "line one line two");
}

#[test]
fn replacing_a_multiline_body_drops_every_old_line() {
    // A local edit replaces the whole body block, not merely its last line,
    // while the quoted header, timestamp and sender tag stay untouched.
    let line = "↳ Bob: earlier text\n12:34 [Alice] line one\nline two\nline three";
    assert_eq!(
        with_body(line, "replacement"),
        "↳ Bob: earlier text\n12:34 [Alice] replacement"
    );
}

#[test]
fn undecodable_storage_json_is_ignored() {
    assert_eq!(decode("not json"), None);
}

#[test]
fn flutter_reply_preview_chars_match_rust() {
    // Flutter's hand-written `replyPreviewLine` bounds a quote to the same
    // number of characters as Rust's `preview_line`, so a snapshot frozen on
    // either frontend renders identically. Guard the Dart constant against the
    // canonical Rust bound (the unit tests are a child module of `reply`, so
    // the private const is in scope).
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("apps/flutter_app/lib/src/util/reply.dart"),
    )
    .expect("reply.dart is readable");
    let line = source
        .lines()
        .find(|line| line.contains("replyPreviewChars"))
        .expect("replyPreviewChars declaration is present");
    let dart_value: usize = line
        .split_once("= ")
        .and_then(|(_, rest)| rest.split(';').next())
        .and_then(|value| value.trim().parse().ok())
        .expect("replyPreviewChars is a usize literal");
    assert_eq!(dart_value, crate::reply::PREVIEW_CHARS);
}
