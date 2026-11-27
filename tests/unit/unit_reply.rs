use crate::reply::{
    ReplyRef, canonicalized_option, decode, display_body, encode, from_display_line, preview_line,
    quoted_header, split_display_line, with_body,
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
fn serde_ingress_canonicalizes_both_display_strings() {
    // Wire, web and DB ingress all go through serde. A peer can put newlines in
    // either display field; neither may survive into a transcript line.
    let reply: ReplyRef = serde_json::from_str(
        r#"{"msgId":"source-1","sender":"Alice\nInjected","preview":"line one\nline two"}"#,
    )
    .expect("deserialize crafted reply");
    assert_eq!(
        reply,
        ReplyRef {
            msg_id: "source-1".into(),
            sender: "Alice Injected".into(),
            preview: "line one line two".into(),
        }
    );
}

#[test]
fn wire_payload_deserialize_canonicalizes_nested_reply() {
    // The actual chat wire structs use JSON serde, so a nested reply snapshot
    // is covered by the same field-level canonicalization.
    let wire: crate::behavior::BroadcastMessage = serde_json::from_str(
        r#"{"content":"body","reply_to":{"msgId":"source-2","sender":"Bob\nInjected","preview":"first\nsecond"}}"#,
    )
    .expect("deserialize wire broadcast");
    assert_eq!(
        wire.reply_to,
        Some(ReplyRef {
            msg_id: "source-2".into(),
            sender: "Bob Injected".into(),
            preview: "first second".into(),
        })
    );
}

#[test]
fn storage_encode_canonicalizes_a_non_serde_snapshot() {
    // FRB's generated decoder writes public fields directly, bypassing the
    // constructor and serde. Persistence must still flatten both strings.
    let crafted = ReplyRef {
        msg_id: "source-3".into(),
        sender: "Alice\nInjected".into(),
        preview: "line one\nline two".into(),
    };
    let raw = serde_json::to_string(&crafted).expect("serialize crafted reply");
    assert!(raw.contains("\\n"), "crafted snapshot keeps its newline");

    let stored = encode(&crafted).expect("canonical storage json");
    assert!(!stored.contains("\\n"), "stored quote is single-line");
    assert_eq!(decode(&stored), Some(crafted.canonicalized()));
}

#[test]
fn quoted_header_canonicalizes_a_bypassed_snapshot() {
    let crafted = ReplyRef {
        msg_id: "source-4".into(),
        sender: "Alice\nInjected".into(),
        preview: "line one\nline two".into(),
    };
    assert_eq!(
        quoted_header(&crafted),
        "↳ Alice Injected: line one line two"
    );
}

#[test]
fn optional_canonicalization_handles_none_and_some() {
    assert_eq!(canonicalized_option(None), None);
    let crafted = ReplyRef {
        msg_id: "source-5".into(),
        sender: "Alice\nInjected".into(),
        preview: "line one\nline two".into(),
    };
    assert_eq!(
        canonicalized_option(Some(crafted.clone())),
        Some(crafted.canonicalized())
    );
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
fn flutter_delegates_reply_drafts_to_rust() {
    // The Dart helper must ask the node (and therefore Rust) to freeze a draft
    // instead of duplicating the sender, attachment and 80-character preview
    // rules. Guarding the boundary keeps both frontends on one canonical rule.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("apps/flutter_app/lib/src/util/reply.dart"),
    )
    .expect("reply.dart is readable");
    assert!(
        source.contains("nodeApi.replyDraft("),
        "reply.dart delegates draft construction to Rust"
    );
    for duplicated in ["replyPreviewChars", "replyPreviewLine", "replyPreviewBody"] {
        assert!(
            !source.contains(duplicated),
            "{duplicated} must not be reimplemented in Dart"
        );
    }
    // The canonical bound still exists in Rust (the unit tests are a child
    // module of `reply`, so the private const is in scope).
    assert_eq!(crate::reply::PREVIEW_CHARS, 80);
}
