#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction,
    clippy::used_underscore_binding,
    clippy::significant_drop_tightening
)]

use super::*;
use serde_json::json;
use serial_test::serial;
use std::collections::BTreeSet;
use std::future::IntoFuture as _;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tower::ServiceExt as _;

// --- helpers ---

fn test_state(
    credentials: Option<Credentials>,
) -> (WebState, tokio::sync::broadcast::Sender<Value>) {
    let (events, _) = tokio::sync::broadcast::channel::<Value>(EVENT_CHANNEL_CAPACITY);
    let state = WebState {
        events: events.clone(),
        credentials,
    };
    (state, events)
}

fn credentials() -> Credentials {
    Credentials {
        user: "p2p".to_owned(),
        password: "s3cret".to_owned(),
    }
}

/// `Authorization: Basic …` for `user:password`.
fn basic_header(user: &str, password: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "Basic {}",
        BASE64.encode(format!("{user}:{password}"))
    ))
    .expect("basic header")
}

/// `Cookie` header carrying the given `p2p_web_session` value.
fn cookie_header(value: &str) -> HeaderValue {
    HeaderValue::from_str(&format!("other=1; {SESSION_COOKIE}={value}; x=2"))
        .expect("cookie header")
}

/// Parse one request through the frame handler and return the response.
async fn frame(request: &str) -> Value {
    serde_json::from_str(&answer_frame(request).await).expect("response frame is JSON")
}

fn http_request(method: &str, uri: &str, body: Vec<u8>) -> Request {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(axum::body::Body::from(body))
        .expect("test request builds")
}

async fn response_json(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body reads");
    serde_json::from_slice(&bytes).expect("response body is JSON")
}

async fn response_bytes(response: Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body reads")
        .to_vec()
}

/// Fresh tempdir DB for the current test (caller holds the DB lock).
fn init_test_db(dir: &tempfile::TempDir) -> String {
    crate::reset_db_url();
    let db_path = dir.path().join("web.sqlite");
    crate::mobile_api::init_mobile_database(db_path.to_string_lossy().into_owned())
        .expect("mobile init");
    db_path.to_string_lossy().into_owned()
}

fn db_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Run async request work from a sync `#[test]` that holds the DB lock
/// (Mutex guards must never live across an await in an async test).
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("test runtime")
        .block_on(future)
}

// --- frame protocol ---

#[tokio::test]
async fn invalid_frame_is_answered_with_an_error_keyed_to_id_zero() {
    let response = frame("this is not json").await;
    assert_eq!(response["id"], 0);
    assert!(
        response["err"]
            .as_str()
            .expect("err string")
            .starts_with("invalid frame:"),
        "unexpected err: {response}"
    );
}

#[tokio::test]
async fn missing_method_is_an_error_with_echoed_id() {
    let response = frame(r#"{"id": 41, "params": {}}"#).await;
    assert_eq!(response, json!({"id": 41, "err": "missing method"}));
}

#[tokio::test]
async fn unknown_method_is_an_error() {
    let response = frame(r#"{"id": 7, "method": "frobnicate", "params": null}"#).await;
    assert_eq!(
        response,
        json!({"id": 7, "err": "unknown method: frobnicate"})
    );
}

#[tokio::test]
async fn params_are_camel_case_and_malformed_ones_are_rejected() {
    // camelCase key: accepted.
    let response =
            frame(r#"{"id": 1, "method": "isAtBottom", "params": {"scrollOffset": 500, "total": 580, "visible": 80}}"#)
                .await;
    assert_eq!(response, json!({"id": 1, "ok": true}));

    // snake_case key (the pre-web shape): rejected, not silently defaulted.
    let response =
            frame(r#"{"id": 2, "method": "isAtBottom", "params": {"scroll_offset": 500, "total": 580, "visible": 80}}"#)
                .await;
    assert_eq!(response["id"], 2);
    assert!(
        response["err"]
            .as_str()
            .expect("err string")
            .starts_with("invalid params:"),
        "unexpected err: {response}"
    );

    // Missing params for a params-taking method: also an error.
    let response = frame(r#"{"id": 3, "method": "formatTimeHhmm", "params": null}"#).await;
    assert_eq!(response["id"], 3);
    assert!(
        response["err"]
            .as_str()
            .expect("err")
            .starts_with("invalid params:")
    );
}

#[tokio::test]
async fn pure_helpers_answer_over_the_frame_protocol() {
    // Times: `HH:MM` slice of a timestamp, whole string when too short.
    let response =
        frame(r#"{"id": 1, "method": "formatTimeHhmm", "params": {"dt": "2026-10-19 08:42:00"}}"#)
            .await;
    assert_eq!(response, json!({"id": 1, "ok": "08:42"}));
    let response =
        frame(r#"{"id": 2, "method": "formatTimeHhmm", "params": {"dt": "08:42"}}"#).await;
    assert_eq!(response, json!({"id": 2, "ok": "08:42"}));

    // One canonical last-seen parser (seconds → millis, 0 on garbage).
    let response = frame(
        r#"{"id": 3, "method": "parseLastSeenMs", "params": {"lastSeen": "1970-01-01 00:00:10"}}"#,
    )
    .await;
    assert_eq!(response, json!({"id": 3, "ok": 10_000}));
    let response =
        frame(r#"{"id": 4, "method": "parseLastSeenMs", "params": {"lastSeen": "not-a-date"}}"#)
            .await;
    assert_eq!(response, json!({"id": 4, "ok": 0}));

    // The quick palette is Rust-owned so the TUI and Flutter cannot drift.
    let response = frame(r#"{"id": 11, "method": "quickReactions", "params": null}"#).await;
    assert_eq!(
        response,
        json!({"id": 11, "ok": ["👍", "❤️", "😂", "🎉", "😮", "😢"]})
    );

    // Network-size buckets (the label rule lives in Rust only).
    let response =
        frame(r#"{"id": 5, "method": "networkSizeLabel", "params": {"peerCount": 2}}"#).await;
    assert_eq!(response, json!({"id": 5, "ok": "Small"}));
    let response =
        frame(r#"{"id": 6, "method": "networkSizeLabel", "params": {"peerCount": 40}}"#).await;
    assert_eq!(response, json!({"id": 6, "ok": "Large"}));
    // Both boundaries follow the live connected count, not the historical
    // average used to tune gossipsub.
    let response =
        frame(r#"{"id": 9, "method": "networkSizeLabel", "params": {"peerCount": 4}}"#).await;
    assert_eq!(response, json!({"id": 9, "ok": "Medium"}));
    let response =
        frame(r#"{"id": 10, "method": "networkSizeLabel", "params": {"peerCount": 15}}"#).await;
    assert_eq!(response, json!({"id": 10, "ok": "Medium"}));

    // Nickname validation (canonical Rust rule).
    let response =
        frame(r#"{"id": 7, "method": "validateNickname", "params": {"name": "alice-1"}}"#).await;
    assert_eq!(response, json!({"id": 7, "ok": true}));
    let response =
        frame(r#"{"id": 8, "method": "validateNickname", "params": {"name": ""}}"#).await;
    assert_eq!(response, json!({"id": 8, "ok": false}));

    // Reply drafts use the same canonical rules on the web transport.
    let response = frame(
            r#"{"id": 12, "method": "replyDraft", "params": {"msgId": "m1", "senderNickname": "Alice", "content": "  see\t attached  ", "attachment": {"fileId": "abc", "name": "notes.txt", "size": 2048, "sizeLabel": "2 KiB", "chip": "[file] notes.txt (2 KiB)", "mime": null}}}"#,
        )
        .await;
    assert_eq!(
        response,
        json!({"id": 12, "ok": {"msgId": "m1", "sender": "Alice", "preview": "see attached [file] notes.txt (2 KiB)"}})
    );
    let response =
        frame(r#"{"id": 13, "method": "replyDraft", "params": {"msgId": "", "content": "hello"}}"#)
            .await;
    assert_eq!(response, json!({"id": 13, "ok": null}));
}

#[test]
fn documented_params_methods_are_routed() {
    // Every method that takes params must parse (and then fail) rather
    // than answer "unknown method"/"not routed": parse failures happen
    // before any node/DB work, so this needs no test DB.
    for method in [
        "startNode",
        "sortPeers",
        "loadBroadcastMessages",
        "loadDmMessages",
        "deleteMessage",
        "editMessage",
        "sendTyping",
        "toggleReaction",
        "getReactions",
        "validateEmoji",
        "saveOutgoingBroadcast",
        "saveOutgoingDm",
        "saveIncomingMessage",
        "createGroup",
        "loadGroupMessages",
        "saveOutgoingGroup",
        "acceptGroupInvite",
        "deleteGroup",
        "setSelfNickname",
        "setPeerLocalNickname",
        "validateNickname",
        "networkSizeLabel",
        "getPeerQuality",
        "sendDeviceBindingIntro",
        "acceptDeviceBinding",
        "getUserDevices",
        "sortContacts",
        "startCall",
        "sendCallSignal",
        "startCallMedia",
        "sendCallMedia",
        "setCallMediaMute",
        "addNetworkPeer",
        "removeNetworkPeer",
        "setLanOnly",
        "setTheme",
        "setNotificationSettings",
        "blockPeer",
        "unblockPeer",
        "createShare",
        "getShare",
        "bindShare",
        "deleteShare",
        "setShareVisibility",
        "listShareGrants",
        "setShareGrant",
        "removeShareGrant",
        "loadShareManifest",
        "saveShareManifest",
        "acceptShareOffer",
        "snapshotShare",
        "requestPeerShares",
        "requestShareManifest",
        "readvertiseShare",
        "publishShareSnapshot",
        "formatTimeHhmm",
        "isAtBottom",
        "parseLastSeenMs",
        "replyDraft",
    ] {
        let err =
            methods::dispatch(method, Value::Null).expect_err("params-less call is a parse error");
        assert!(
            !err.starts_with("unknown method") && !err.contains("not routed"),
            "{method} was not routed: {err}"
        );
        assert!(err.starts_with("invalid params:"), "{method}: {err}");
    }

    // A routed pure helper returns its value (proves the arm is live).
    assert_eq!(
        methods::dispatch("formatTimeHhmm", json!({"dt": "08:42"})).expect("pure helper"),
        json!("08:42")
    );
}

#[test]
#[serial(db)]
fn save_incoming_message_frame_carries_canonical_reply_to_and_attachment() {
    let _guard = db_lock();
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir
        .path()
        .join("web-reply.sqlite")
        .to_string_lossy()
        .into_owned();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .expect("test runtime");

    let (response, persisted) = rt.block_on(async {
            tokio::task::spawn_blocking({
                let db_path = db_path.clone();
                move || {
                    crate::reset_db_url();
                    crate::mobile_api::init_mobile_database(db_path).expect("mobile init");
                }
            })
            .await
            .expect("init task");

            let response = frame(
                r#"{"id": 14, "method": "saveIncomingMessage", "params": {
                    "content": "reply body", "peerId": "peer-reply", "isDirect": false,
                    "latency": "5ms", "msgId": "incoming-9", "encrypted": true,
                    "replyTo": {"msgId": "old-9", "sender": "Alice\nInjected", "preview": "earlier\ntext"},
                    "attachment": {"fileId": "abc123", "name": "notes.txt", "size": 2048,
                                    "sizeLabel": "2 KiB", "chip": "[file] notes.txt (2 KiB)",
                                    "mime": "text/plain"}
                }}"#,
            )
            .await;

            let persisted = tokio::task::spawn_blocking(|| {
                crate::messages::load_messages(crate::CHAT_TOPIC, 100)
                    .expect("history")
                    .into_iter()
                    .find(|m| m.msg_id.as_deref() == Some("incoming-9"))
                    .map(|m| (m.msg_id, m.reply_to_json, m.attachment_json, m.encrypted))
            })
            .await
            .expect("read task");
            (response, persisted)
        });

    assert_eq!(response["id"], 14);
    assert_eq!(response["ok"]["msgId"], "incoming-9");
    assert_eq!(response["ok"]["encrypted"], true);
    assert_eq!(response["ok"]["replyTo"]["msgId"], "old-9");
    assert_eq!(response["ok"]["replyTo"]["sender"], "Alice Injected");
    assert_eq!(response["ok"]["replyTo"]["preview"], "earlier text");
    assert_eq!(response["ok"]["attachment"]["fileId"], "abc123");
    assert_eq!(
        response["ok"]["attachment"]["chip"],
        "[file] notes.txt (2 KiB)"
    );
    let (msg_id, reply_json, attachment_json, encrypted) =
        persisted.expect("row saved through the web dispatch");
    assert_eq!(msg_id.as_deref(), Some("incoming-9"));
    assert_eq!(encrypted, 1, "the US-042 flag must be persisted");
    assert_eq!(
        crate::reply::decode(&reply_json.expect("reply snapshot")),
        Some(crate::ReplyRef::new(
            "old-9".to_string(),
            "Alice Injected".to_string(),
            "earlier text"
        ))
    );
    let attachment = crate::files::attachment_from_json(
        &attachment_json.expect("attachment descriptor persisted"),
    )
    .expect("valid canonical attachment");
    assert_eq!(attachment.file_id, "abc123");
    assert_eq!(attachment.size, 2048);
}

#[test]
#[serial(db)]
fn save_outgoing_never_reads_a_client_supplied_attachment_path() {
    let _guard = db_lock();
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir
        .path()
        .join("web-path.sqlite")
        .to_string_lossy()
        .into_owned();
    // A real, readable file with recognizable contents. Before the fix the
    // web surface's `attachmentPath` fallback fed this straight to
    // `std::fs::read`, giving any authenticated client an arbitrary-file-
    // read + p2p exfiltration primitive. The web client uploads bytes via
    // `/api/upload` and sends only the resulting descriptor.
    let secret = dir.path().join("secret.bin");
    std::fs::write(&secret, b"TOP-SECRET-SERVER-FILE").expect("write secret");
    let secret_path = secret.to_string_lossy().into_owned();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .expect("test runtime");

    let (response, saved) = rt.block_on(async {
        tokio::task::spawn_blocking({
            let db_path = db_path.clone();
            move || {
                crate::reset_db_url();
                crate::mobile_api::init_mobile_database(db_path).expect("mobile init");
            }
        })
        .await
        .expect("init task");

        let request = json!({
            "id": 22,
            "method": "saveOutgoingBroadcast",
            "params": { "content": "hello", "attachmentPath": secret_path }
        })
        .to_string();
        let response = frame(&request).await;

        let saved = tokio::task::spawn_blocking(|| {
            crate::messages::load_messages(crate::CHAT_TOPIC, 100)
                .expect("history")
                .into_iter()
                .find(|m| m.content == "hello")
                .map(|m| (m.attachment_json, m.content))
        })
        .await
        .expect("read task");
        (response, saved)
    });

    assert_eq!(response["id"], 22);
    assert!(
        response["ok"]["msgId"].is_string(),
        "message must still save without a path: {response}"
    );
    let (attachment_json, _) = saved.expect("message saved through the web dispatch");
    assert!(
        attachment_json.is_none(),
        "a client-supplied attachmentPath must never read a server-local file: {attachment_json:?}"
    );
}

#[test]
#[serial(db)]
fn delete_message_frame_reaches_the_local_delete_helper() {
    let _guard = db_lock();
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("web.sqlite").to_string_lossy().into_owned();
    // Handlers do their DB work on `spawn_blocking` workers and the DB URL
    // is per-thread in tests: pin the blocking pool to one thread and
    // initialize the DB there so seed, frame, and assertions share it.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .expect("test runtime");

    let (response, bad_params, incoming_response, hidden, incoming_id) = rt.block_on(async {
            let (sent_id, incoming_id) = tokio::task::spawn_blocking({
                let db_path = db_path.clone();
                move || {
                    crate::reset_db_url();
                    crate::mobile_api::init_mobile_database(db_path).expect("mobile init");
                    let sent = crate::messages::save_message(
                        "mine",
                        None,
                        crate::CHAT_TOPIC,
                        false,
                        None,
                    )
                    .expect("save sent");
                    crate::messages::mark_message_sent(sent.id).expect("mark sent");
                    let incoming = crate::messages::save_message(
                        "theirs",
                        Some("peer-a"),
                        crate::CHAT_TOPIC,
                        false,
                        None,
                    )
                    .expect("save incoming");
                    (sent.id, incoming.id)
                }
            })
            .await
            .expect("seed task");

            let response = frame(&format!(
                r#"{{"id": 11, "method": "deleteMessage", "params": {{"messageId": {sent_id}, "isGroup": false}}}}"#
            ))
            .await;
            let bad_params = frame(&format!(
                r#"{{"id": 12, "method": "deleteMessage", "params": {{"messageId": {sent_id}}}}}"#
            ))
            .await;
            let incoming_response = frame(&format!(
                r#"{{"id": 13, "method": "deleteMessage", "params": {{"messageId": {incoming_id}, "isGroup": false}}}}"#
            ))
            .await;
            let hidden = tokio::task::spawn_blocking(move || {
                crate::messages::load_messages(crate::CHAT_TOPIC, 100)
                    .expect("history")
                    .iter()
                    .any(|m| m.id == sent_id)
            })
            .await
            .expect("read task");
            (response, bad_params, incoming_response, hidden, incoming_id)
        });

    assert_eq!(response, json!({"id": 11, "ok": null}));
    assert!(
        bad_params["err"]
            .as_str()
            .expect("err")
            .starts_with("invalid params:"),
        "unexpected err: {bad_params}"
    );
    assert_eq!(
        incoming_response["err"].as_str().expect("err"),
        format!("no deletable sent message with id {incoming_id}")
    );
    assert!(!hidden, "the deleted row must disappear from history");
}

/// FRB wires that deliberately have no JSON arm.
///
/// The web server owns node startup and event pushing, so Flutter's
/// bootstrap/poll and peer-id probes have no browser equivalent. Browser
/// attachment downloads go through `GET /api/files/{file_id}` instead of a
/// local filesystem path.
const NATIVE_ONLY_WEB_EXEMPTIONS: [&str; 4] = [
    "initMobileDatabase",
    "pollEvent",
    "attachmentFilePath",
    "getNodePeerId",
];

/// Convert an FRB wire function's `snake_case` name to its Dart method name.
fn snake_case_to_camel_case(name: &str) -> String {
    let mut parts = name.split('_');
    let mut camel = parts.next().unwrap_or_default().to_owned();
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            camel.push(first.to_ascii_uppercase());
            camel.push_str(chars.as_str());
        }
    }
    camel
}

/// Parse the generated wire function names from `frb_generated.rs`.
fn frb_wire_methods(generated: &str) -> BTreeSet<String> {
    generated
        .lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix("fn wire__crate__")?;
            let (_, name) = rest.rsplit_once("__")?;
            let snake = name.strip_suffix("_impl(")?;
            Some(snake_case_to_camel_case(snake))
        })
        .collect()
}

/// Read the generated bridge source from the crate root.
fn generated_frb_source() -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/frb_generated.rs"),
    )
    .expect("frb_generated.rs is readable")
}

/// Read just the dispatch portion of `methods.rs`, without its test module.
fn production_methods_source() -> String {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/web/methods.rs"),
    )
    .expect("methods.rs is readable");
    source
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(source.as_str())
        .to_owned()
}

#[test]
fn frb_wire_method_parser_handles_modules_and_snake_case() {
    let generated = r"
fn wire__crate__api__accept_device_binding_impl(
fn wire__crate__mobile_node__get_node_peer_id_impl(
fn wire__crate__mobile_api__parse_last_seen_ms_impl(
";
    let parsed = frb_wire_methods(generated);
    assert_eq!(parsed.len(), 3, "unexpected parse: {parsed:?}");
    assert!(parsed.contains("acceptDeviceBinding"));
    assert!(parsed.contains("getNodePeerId"));
    assert!(parsed.contains("parseLastSeenMs"));
}

#[test]
fn every_frb_wire_method_has_a_web_arm_or_a_native_only_reason() {
    let wire_methods = frb_wire_methods(&generated_frb_source());
    assert!(
        wire_methods.contains("acceptDeviceBinding"),
        "failed to parse FRB wire functions from src/frb_generated.rs"
    );
    for exemption in NATIVE_ONLY_WEB_EXEMPTIONS {
        assert!(
            wire_methods.contains(exemption),
            "native-only exemption {exemption} is no longer FRB-exposed; remove it from the list"
        );
    }

    let methods = production_methods_source();
    for method in wire_methods {
        if NATIVE_ONLY_WEB_EXEMPTIONS.contains(&method.as_str()) {
            continue;
        }
        assert!(
            methods.contains(&format!("\"{method}\"")),
            "FRB method {method} has no web dispatch arm"
        );
    }
}

#[test]
fn documented_no_params_methods_stay_in_the_dispatch_table() {
    // These arms take no params, so probing them through `dispatch` would
    // EXECUTE them (reading the DB, starting a node for `startNodeAuto`,
    // stopping the shared node for `stopNode`) — a drift guard on the
    // source instead.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/web/methods.rs"),
    )
    .expect("methods.rs is readable");
    for method in [
        "getMobilePeerStatus",
        "startNodeAuto",
        "stopNode",
        "getLogs",
        "clearLogs",
        "getKnownPeers",
        "listGroups",
        "getContactsWithStats",
        "getPeersWithStats",
        "getNetworkDetails",
        "getMyUserId",
        "listNetworkPeers",
        "getLanOnly",
        "quickReactions",
        "getBroadcastUnreadCount",
        "markBroadcastViewed",
        "getTheme",
        "getNotificationSettings",
        "listBlockedPeers",
        "stopCallMedia",
        "listShares",
        "getLocationDefaults",
        "listLocationRecipients",
    ] {
        assert!(
            source.contains(&format!("\"{method}\"")),
            "{method} missing from the dispatch table"
        );
    }
}

// --- auth ---

#[test]
fn basic_credentials_must_match_the_configured_pair() {
    let credentials = credentials();
    let mut headers = HeaderMap::new();
    assert!(
        !basic_credentials_match(&credentials, &headers),
        "no header must be denied"
    );

    headers.insert(header::AUTHORIZATION, basic_header("p2p", "s3cret"));
    assert!(basic_credentials_match(&credentials, &headers));

    headers.insert(header::AUTHORIZATION, basic_header("p2p", "wrong"));
    assert!(!basic_credentials_match(&credentials, &headers));

    headers.insert(header::AUTHORIZATION, basic_header("other", "s3cret"));
    assert!(
        !basic_credentials_match(&credentials, &headers),
        "the username is part of the credential"
    );

    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer s3cret"),
    );
    assert!(
        !basic_credentials_match(&credentials, &headers),
        "only Basic is accepted"
    );
}

#[test]
fn ws_origin_matches_host_only_rejects_present_mismatches() {
    let mut headers = HeaderMap::new();
    // Non-browser clients send no Origin and must be allowed.
    assert!(ws_origin_matches_host(&headers));

    headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:8080"));
    // Still absent: allowed even with a Host present.
    assert!(ws_origin_matches_host(&headers));

    headers.insert(
        header::ORIGIN,
        HeaderValue::from_static("http://127.0.0.1:8080"),
    );
    assert!(ws_origin_matches_host(&headers));

    // Scheme is irrelevant, authority (case-insensitive) is what matters.
    headers.insert(
        header::ORIGIN,
        HeaderValue::from_static("ws://127.0.0.1:8080"),
    );
    assert!(ws_origin_matches_host(&headers));
    headers.insert(
        header::ORIGIN,
        HeaderValue::from_static("https://LOCALHOST:8080"),
    );
    assert!(!ws_origin_matches_host(&headers));

    // A foreign origin is the hijack case: rejected.
    headers.insert(
        header::ORIGIN,
        HeaderValue::from_static("https://evil.example"),
    );
    assert!(!ws_origin_matches_host(&headers));

    // Malformed Origin (no scheme separator) is rejected, not allowed.
    headers.insert(header::ORIGIN, HeaderValue::from_static("null"));
    assert!(!ws_origin_matches_host(&headers));
}

#[test]
fn basic_decoding_needs_the_exact_scheme_and_valid_base64() {
    let mut headers = HeaderMap::new();
    assert_eq!(basic_credentials(&headers), None);

    headers.insert(header::AUTHORIZATION, basic_header("p2p", "s3cret"));
    assert_eq!(basic_credentials(&headers).as_deref(), Some("p2p:s3cret"));

    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Basic not-base64!"),
    );
    assert_eq!(basic_credentials(&headers), None);

    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Basic cDpw"), // "p:p"
    );
    assert_eq!(basic_credentials(&headers).as_deref(), Some("p:p"));
}

#[test]
fn session_cookie_round_trips_and_rejects_forgeries() {
    let credentials = credentials();
    let value = session_cookie_value(&credentials);

    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie_header(&value));
    assert!(session_cookie_is_valid(&credentials, &headers));

    // A different password keys a different MAC.
    let other = Credentials {
        user: "p2p".to_owned(),
        password: "different".to_owned(),
    };
    assert!(!session_cookie_is_valid(&other, &headers));

    // Tampering with the expiry invalidates the MAC.
    let (expiry, mac) = value.split_once('.').expect("expiry.mac");
    let extended: i64 = expiry
        .parse::<i64>()
        .expect("expiry")
        .saturating_add(86_400);
    let forged = format!("{extended}.{mac}");
    let mut forged_headers = HeaderMap::new();
    forged_headers.insert(header::COOKIE, cookie_header(&forged));
    assert!(!session_cookie_is_valid(&credentials, &forged_headers));

    // An expired session is rejected even with a correct MAC.
    let stale_at = chrono::Utc::now().timestamp().saturating_sub(1);
    let stale = format!(
        "{stale_at}.{}",
        BASE64_URL.encode(session_mac(&credentials, stale_at))
    );
    let mut stale_headers = HeaderMap::new();
    stale_headers.insert(header::COOKIE, cookie_header(&stale));
    assert!(!session_cookie_is_valid(&credentials, &stale_headers));

    // Garbage is rejected rather than panicking.
    for junk in ["", "nodot", "abc.def", "1.", ".1"] {
        let mut junk_headers = HeaderMap::new();
        junk_headers.insert(header::COOKIE, cookie_header(junk));
        assert!(
            !session_cookie_is_valid(&credentials, &junk_headers),
            "{junk:?} must not validate"
        );
    }
}

#[test]
fn session_cookie_attributes_are_browser_safe() {
    let cookie = session_cookie(&credentials());
    assert!(cookie.starts_with(&format!("{SESSION_COOKIE}=")));
    for attribute in ["Path=/", "HttpOnly", "SameSite=Strict", "Max-Age="] {
        assert!(cookie.contains(attribute), "missing {attribute}: {cookie}");
    }
    assert!(
        !cookie.contains("Secure"),
        "the server has no TLS of its own; `Secure` would break plain-HTTP LAN use"
    );
}

#[test]
fn cookie_value_parsing_is_exact() {
    let mut headers = HeaderMap::new();
    assert_eq!(cookie_value(&headers, SESSION_COOKIE), None);

    headers.insert(
        header::COOKIE,
        HeaderValue::from_static("a=1; p2p_web_session=abc; b=2"),
    );
    assert_eq!(
        cookie_value(&headers, SESSION_COOKIE).as_deref(),
        Some("abc")
    );
    assert_eq!(cookie_value(&headers, "a").as_deref(), Some("1"));

    headers.insert(
        header::COOKIE,
        HeaderValue::from_static("notp2p_web_session=abc"),
    );
    assert_eq!(cookie_value(&headers, SESSION_COOKIE), None);
}

// --- HTTP routes ---

#[tokio::test]
async fn status_is_open_without_credentials_and_gated_with_them() {
    let (state, _events) = test_state(None);
    let response = build_router(state, None)
        .oneshot(http_request("GET", "/api/status", Vec::new()))
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["service"], "p2p-chat-web");

    let (state, _events) = test_state(Some(credentials()));
    let denied = build_router(state.clone(), None)
        .oneshot(http_request("GET", "/api/status", Vec::new()))
        .await
        .expect("router responds");
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        denied
            .headers()
            .get(header::WWW_AUTHENTICATE)
            .and_then(|v| v.to_str().ok()),
        Some(AUTH_CHALLENGE),
        "the challenge is what makes a browser prompt"
    );

    let mut wrong = http_request("GET", "/api/status", Vec::new());
    wrong
        .headers_mut()
        .insert(header::AUTHORIZATION, basic_header("p2p", "wrong"));
    let denied = build_router(state.clone(), None)
        .oneshot(wrong)
        .await
        .expect("router responds");
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

    let mut authorized = http_request("GET", "/api/status", Vec::new());
    authorized
        .headers_mut()
        .insert(header::AUTHORIZATION, basic_header("p2p", "s3cret"));
    let allowed = build_router(state.clone(), None)
        .oneshot(authorized)
        .await
        .expect("router responds");
    assert_eq!(allowed.status(), StatusCode::OK);
    let set_cookie = allowed
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("a successful login mints the session cookie")
        .to_owned();
    assert!(set_cookie.starts_with(&format!("{SESSION_COOKIE}=")));

    // The cookie alone (no Authorization header) is now enough.
    let mut with_cookie = http_request("GET", "/api/status", Vec::new());
    let value = set_cookie
        .split(';')
        .next()
        .expect("name=value")
        .split_once('=')
        .expect("name=value")
        .1;
    with_cookie
        .headers_mut()
        .insert(header::COOKIE, cookie_header(value));
    let allowed = build_router(state, None)
        .oneshot(with_cookie)
        .await
        .expect("router responds");
    assert_eq!(allowed.status(), StatusCode::OK);
}

#[tokio::test]
async fn static_assets_are_gated_too() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("index.html"), "<html>app shell</html>")
        .expect("write index.html");
    let assets = dir.path().to_path_buf();

    let (state, _events) = test_state(Some(credentials()));
    let denied = build_router(state.clone(), Some(&assets))
        .oneshot(http_request("GET", "/", Vec::new()))
        .await
        .expect("router responds");
    assert_eq!(
        denied.status(),
        StatusCode::UNAUTHORIZED,
        "the SPA shell must not be readable before login"
    );

    let mut authorized = http_request("GET", "/", Vec::new());
    authorized
        .headers_mut()
        .insert(header::AUTHORIZATION, basic_header("p2p", "s3cret"));
    let allowed = build_router(state, Some(&assets))
        .oneshot(authorized)
        .await
        .expect("router responds");
    assert_eq!(allowed.status(), StatusCode::OK);
    assert!(String::from_utf8_lossy(&response_bytes(allowed).await).contains("app shell"));
}

#[test]
#[serial(db)]
fn upload_then_download_round_trips_the_blob() {
    let _guard = db_lock();
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("web.sqlite");
    let data = b"hello web attachments".to_vec();
    let expected_file_id = crate::files::hash_bytes(&data);

    // Handlers do their DB/file work on `spawn_blocking` workers, and the
    // DB URL is per-thread in tests — pin the blocking pool to a single
    // thread and initialize the DB there so every handler (and the final
    // blob assertions) observe the same database.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .expect("test runtime");

    let (status, downloaded, blob, reuploaded_id, expected_file_id) = rt.block_on(async {
        let path = db_path.to_string_lossy().into_owned();
        tokio::task::spawn_blocking(move || {
            crate::reset_db_url();
            crate::mobile_api::init_mobile_database(path).expect("mobile init");
        })
        .await
        .expect("init task");

        let router = build_router(test_state(None).0, None);
        let upload = router
            .clone()
            .oneshot(http_request(
                "POST",
                "/api/upload?name=notes.txt",
                data.clone(),
            ))
            .await
            .expect("router responds");
        assert_eq!(upload.status(), StatusCode::OK);
        let body = response_json(upload).await;
        let attachment = &body["attachment"];
        assert_eq!(attachment["file_id"], expected_file_id);
        assert_eq!(attachment["name"], "notes.txt");
        assert_eq!(attachment["size"], data.len());
        assert_eq!(attachment["mime"], "text/plain");

        // The download route returns the bytes with the stored metadata.
        let download = router
            .oneshot(http_request(
                "GET",
                &format!("/api/files/{expected_file_id}"),
                Vec::new(),
            ))
            .await
            .expect("router responds");
        assert_eq!(
            download
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("text/plain")
        );
        assert_eq!(
            download
                .headers()
                .get(header::CONTENT_DISPOSITION)
                .and_then(|v| v.to_str().ok()),
            Some("attachment; filename=\"notes.txt\"")
        );
        let status = download.status();
        let downloaded = response_bytes(download).await;

        // Content-address + idempotence, resolved on the same pinned
        // thread the handlers used.
        let checks_data = data.clone();
        let checks_file_id = expected_file_id.clone();
        let (blob, reuploaded_id) = tokio::task::spawn_blocking(move || {
            let blob = crate::files::store_path_for(&checks_file_id).expect("store path");
            let reuploaded_id = crate::files::store_outgoing_bytes("other-name.txt", &checks_data)
                .expect("idempotent store")
                .file_id;
            (blob, reuploaded_id)
        })
        .await
        .expect("file checks");
        (status, downloaded, blob, reuploaded_id, expected_file_id)
    });

    assert_eq!(status, StatusCode::OK);
    assert_eq!(downloaded, data);

    // The blob lives content-addressed under the DB's `.files` dir.
    assert!(blob.exists(), "blob not written: {}", blob.display());
    assert!(
        blob.starts_with(dir.path()),
        "blob escaped the DB dir: {}",
        blob.display()
    );
    // Re-uploading the same bytes resolves to the same content address.
    assert_eq!(reuploaded_id, expected_file_id);
}

#[test]
#[serial(db)]
fn unknown_file_is_not_found() {
    let _guard = db_lock();
    let dir = tempfile::tempdir().expect("tempdir");
    init_test_db(&dir);

    let status = block_on(async {
        build_router(test_state(None).0, None)
            .oneshot(http_request("GET", "/api/files/deadbeef", Vec::new()))
            .await
            .expect("router responds")
            .status()
    });
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[test]
#[serial(db)]
fn oversized_upload_is_rejected_with_bad_request() {
    let _guard = db_lock();
    let dir = tempfile::tempdir().expect("tempdir");
    init_test_db(&dir);
    let oversized =
        vec![0_u8; usize::try_from(crate::files::MAX_ATTACHMENT_SIZE).expect("fits") + 1];

    let (status, body) = block_on(async {
        let response = build_router(test_state(None).0, None)
            .oneshot(http_request("POST", "/api/upload?name=big.bin", oversized))
            .await
            .expect("router responds");
        let status = response.status();
        (status, response_bytes(response).await)
    });
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        String::from_utf8_lossy(&body).contains("too large"),
        "unexpected body: {}",
        String::from_utf8_lossy(&body)
    );
}

#[test]
fn disposition_filename_strips_header_metacharacters() {
    assert_eq!(disposition_filename("notes.txt"), "notes.txt");
    assert_eq!(
        disposition_filename("ev\"il\\name\n.txt"),
        "ev_il_name_.txt"
    );
    assert_eq!(disposition_filename("ünïcode.txt"), "ünïcode.txt");
}

// --- end-to-end WebSocket ---

/// Read frames until a text frame arrives (ping/pong skipped).
async fn next_text<S>(socket: &mut S) -> String
where
    S: futures_util::Stream<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>>
        + Unpin,
{
    loop {
        match socket
            .next()
            .await
            .expect("socket frame")
            .expect("frame ok")
        {
            WsMessage::Text(text) => return text.to_string(),
            WsMessage::Close(_) => panic!("server closed the socket"),
            _ => {}
        }
    }
}

#[tokio::test]
async fn websocket_answers_requests_and_pushes_events() {
    let credentials = credentials();
    let (state, events) = test_state(Some(credentials.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(axum::serve(listener, build_router(state, None)).into_future());

    // The browser cannot set headers on a WebSocket handshake, so it
    // authenticates with the session cookie minted by its login.
    let mut request =
        tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(format!(
            "ws://{addr}/ws"
        ))
        .expect("ws request");
    request.headers_mut().insert(
        "cookie",
        HeaderValue::from_str(&format!(
            "{SESSION_COOKIE}={}",
            session_cookie_value(&credentials)
        ))
        .expect("cookie header"),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request)
        .await
        .expect("ws connect");

    // Request/response with the id echoed back.
    socket
        .send(WsMessage::Text(
            r#"{"id": 5, "method": "formatTimeHhmm", "params": {"dt": "2026-10-19 08:42:00"}}"#
                .into(),
        ))
        .await
        .expect("send");
    assert_eq!(
        serde_json::from_str::<Value>(&next_text(&mut socket).await).expect("json"),
        json!({"id": 5, "ok": "08:42"})
    );

    // An error response is keyed to the same id.
    socket
        .send(WsMessage::Text(
            r#"{"id": 6, "method": "frobnicate", "params": null}"#.into(),
        ))
        .await
        .expect("send");
    assert_eq!(
        serde_json::from_str::<Value>(&next_text(&mut socket).await).expect("json"),
        json!({"id": 6, "err": "unknown method: frobnicate"})
    );

    // Server-side poller results arrive as `{"event": ...}` frames.
    events
        .send(json!({"eventType": "peer_found", "peerId": "peer-1"}))
        .expect("broadcast");
    assert_eq!(
        serde_json::from_str::<Value>(&next_text(&mut socket).await).expect("json"),
        json!({"event": {"eventType": "peer_found", "peerId": "peer-1"}})
    );

    server.abort();
}

#[tokio::test]
async fn websocket_rejects_cross_site_origin_but_allows_same_site() {
    let credentials = credentials();
    let (state, _events) = test_state(Some(credentials.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(axum::serve(listener, build_router(state, None)).into_future());

    let cookie = format!("{SESSION_COOKIE}={}", session_cookie_value(&credentials));

    // A valid session from a foreign page is the hijack case: rejected
    // even though the cookie would otherwise authenticate the handshake.
    let mut hostile =
        tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(format!(
            "ws://{addr}/ws"
        ))
        .expect("ws request");
    hostile
        .headers_mut()
        .insert("cookie", HeaderValue::from_str(&cookie).expect("cookie"));
    hostile
        .headers_mut()
        .insert("origin", HeaderValue::from_static("https://evil.example"));
    assert!(
        tokio_tungstenite::connect_async(hostile).await.is_err(),
        "a cross-site WS handshake must be rejected"
    );

    // The SPA connects from its own origin: allowed.
    let mut same_site =
        tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(format!(
            "ws://{addr}/ws"
        ))
        .expect("ws request");
    same_site
        .headers_mut()
        .insert("cookie", HeaderValue::from_str(&cookie).expect("cookie"));
    same_site.headers_mut().insert(
        "origin",
        HeaderValue::from_str(&format!("http://{addr}")).expect("origin"),
    );
    assert!(
        tokio_tungstenite::connect_async(same_site).await.is_ok(),
        "a same-site WS handshake must still work"
    );

    server.abort();
}

#[tokio::test]
async fn websocket_upgrade_requires_credentials() {
    let (state, _events) = test_state(Some(credentials()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(axum::serve(listener, build_router(state, None)).into_future());

    assert!(
        tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
            .await
            .is_err(),
        "a credential-guarded upgrade must not succeed without a session"
    );

    // A forged cookie is not a session either.
    let mut request =
        tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(format!(
            "ws://{addr}/ws"
        ))
        .expect("ws request");
    request.headers_mut().insert(
        "cookie",
        HeaderValue::from_static("p2p_web_session=4102444800.forged"),
    );
    assert!(
        tokio_tungstenite::connect_async(request).await.is_err(),
        "a forged session cookie must be rejected"
    );

    server.abort();
}
