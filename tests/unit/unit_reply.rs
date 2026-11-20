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
fn undecodable_storage_json_is_ignored() {
    assert_eq!(decode("not json"), None);
}
