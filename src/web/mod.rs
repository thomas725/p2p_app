//! Web frontend server (docs/2026-10-18_web_frontend.md).
//!
//! An axum HTTP+WebSocket server that mirrors the FRB API surface
//! ([`crate::api`]) as JSON methods, so a browser client (the Flutter Web
//! build) drives the exact same node the TUI and mobile frontends drive —
//! the node stays native here; the browser is a pure display client.
//!
//! Wire protocol on `GET /ws`:
//! - request  `{"id": <int>, "method": "<name>", "params": <any>}`
//! - response `{"id": <int>, "ok": <result>}` / `{"id": <int>, "err": "<msg>"}`
//! - push     `{"event": <SwarmEventJson>}` (server-side poller drains the
//!   node's event queue and pushes, replacing the client-side poll loop)

pub(crate) mod methods;

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Path, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router, extract::Query};
use futures_util::{SinkExt as _, StreamExt as _};
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

/// How long the event poller sleeps when the node's queue was empty.
const EVENT_POLL_IDLE: Duration = Duration::from_millis(50);
/// Max events drained per poller pass (a burst is forwarded within one pass).
const EVENT_POLL_BATCH: usize = 100;
/// Broadcast-channel capacity for pushed events (slow clients get dropped
/// frames, never a stalled server).
const EVENT_CHANNEL_CAPACITY: usize = 256;
/// Body-limit slack above [`crate::files::MAX_ATTACHMENT_SIZE`] so a
/// max-size upload is accepted whole (the limit itself is enforced again
/// by [`crate::files::store_outgoing_bytes`]).
const UPLOAD_BODY_SLACK: usize = 1024;

/// Server configuration for [`serve`].
pub struct WebConfig {
    /// Address to bind (`127.0.0.1:8080` by default at the call site).
    pub bind: SocketAddr,
    /// Optional bearer token guarding `/ws` and `/api/*`. Required in
    /// practice for non-loopback binds; loopback-only deployments may omit it.
    pub token: Option<String>,
    /// Optional directory with the `flutter build web` output to serve as
    /// static assets (SPA fallback to `index.html`).
    pub assets: Option<PathBuf>,
}

/// Shared server state (event fan-out + auth token).
#[derive(Clone)]
struct WebState {
    events: tokio::sync::broadcast::Sender<Value>,
    token: Option<String>,
}

/// Serve the web API (and optional static assets) until Ctrl-C.
///
/// Starts the node-event poller that pushes [`crate::api::poll_event`]
/// results to every connected WebSocket client.
///
/// # Errors
/// Returns an error if the listener cannot bind or serving is interrupted.
pub async fn serve(config: WebConfig) -> Result<(), String> {
    let (events, _) = tokio::sync::broadcast::channel::<Value>(EVENT_CHANNEL_CAPACITY);
    spawn_event_poller(events.clone());

    let state = WebState {
        events,
        token: config.token.clone(),
    };
    let app = build_router(state, config.assets.as_ref());

    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .map_err(|e| format!("failed to bind {}: {e}", config.bind))?;
    crate::logging::p2plog_info(format!(
        "web frontend listening on http://{}{}",
        config.bind,
        if config.assets.is_some() {
            " (serving Flutter web assets)"
        } else {
            ""
        }
    ));

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|e| format!("web server error: {e}"))
}

/// Assemble the full router (API routes, auth, CORS, optional static assets)
/// from a prepared [`WebState`]. Split out of [`serve`] so tests can drive
/// the routes directly.
fn build_router(state: WebState, assets: Option<&PathBuf>) -> Router {
    let body_limit = usize::try_from(crate::files::MAX_ATTACHMENT_SIZE)
        .unwrap_or(usize::MAX)
        .saturating_add(UPLOAD_BODY_SLACK);

    let mut app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/status", get(status_handler))
        .route("/api/upload", post(upload_handler))
        .route("/api/files/{file_id}", get(download_handler))
        .layer(DefaultBodyLimit::max(body_limit))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(CorsLayer::permissive());

    if let Some(assets) = assets {
        let index = assets.join("index.html");
        app = app.fallback_service(ServeDir::new(assets).fallback(ServeFile::new(index)));
    }
    // Convert `Router<WebState>` into a servable `Router<()>` (the state is
    // already attached to the middleware; this fills the fallback slot).
    app.with_state(state)
}

/// Drain the node's event queue and push every event as a JSON frame to all
/// connected WebSocket clients. Polls like the Flutter client did (a short
/// idle sleep per empty pass) but forwards instead of waiting to be polled.
fn spawn_event_poller(events: tokio::sync::broadcast::Sender<Value>) {
    tokio::spawn(async move {
        loop {
            let mut pushed_any = false;
            for _ in 0..EVENT_POLL_BATCH {
                let polled = tokio::task::spawn_blocking(crate::api::poll_event).await;
                match polled {
                    Ok(Ok(Some(ev))) => {
                        pushed_any = true;
                        if let Ok(value) = serde_json::to_value(&ev) {
                            let _ = events.send(value);
                        }
                    }
                    // Empty queue, node-not-started, or a panicked poller
                    // call: stop the pass and retry after the idle sleep.
                    Ok(_) | Err(_) => break,
                }
            }
            if !pushed_any {
                tokio::time::sleep(EVENT_POLL_IDLE).await;
            }
        }
    });
}

// --- Auth ---

/// Bearer-token gate for `/ws` and `/api/*`. No token configured → open
/// (the loopback default). Static assets stay reachable either way.
async fn auth_middleware(State(state): State<WebState>, request: Request, next: Next) -> Response {
    if auth_allows(&state, &request) {
        return next.run(request).await;
    }
    (StatusCode::UNAUTHORIZED, "invalid or missing token").into_response()
}

fn auth_allows(state: &WebState, request: &Request) -> bool {
    let Some(expected) = state.token.as_deref() else {
        return true;
    };
    bearer_token(request.headers()).as_deref() == Some(expected)
        || query_token(request.uri().query()).as_deref() == Some(expected)
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    Some(token.to_owned())
}

fn query_token(query: Option<&str>) -> Option<String> {
    query?
        .split('&')
        .find_map(|kv| kv.strip_prefix("token="))
        .map(str::to_owned)
}

// --- WebSocket ---

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<WebState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Serve one client: answer request frames, forward pushed events, exit on
/// disconnect. Event frames interleave with responses on the same socket, so
/// the client demultiplexes on the `id`/`event` discriminant.
async fn handle_socket(socket: WebSocket, state: WebState) {
    let (mut tx, mut rx) = socket.split();
    let mut events = state.events.subscribe();
    loop {
        tokio::select! {
            pushed = events.recv() => {
                match pushed {
                    Ok(value) => {
                        let frame = json!({ "event": value }).to_string();
                        if tx.send(Message::Text(frame.into())).await.is_err() {
                            break;
                        }
                    }
                    // A lagged client misses frames but keeps the stream
                    // (no-op arm); a closed channel means server shutdown.
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = rx.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let response = answer_frame(text.as_str()).await;
                        if tx.send(Message::Text(response.into())).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                    // Ping/pong/binary frames carry no protocol meaning.
                    Some(Ok(_)) => {}
                }
            }
        }
    }
}

/// Parse one request frame and produce the matching response frame. A frame
/// that doesn't parse gets an `err` response keyed to id 0 (or no id at all
/// is still answerable, so a broken client can self-correct).
async fn answer_frame(text: &str) -> String {
    let parsed: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            return json!({ "id": 0, "err": format!("invalid frame: {e}") }).to_string();
        }
    };
    let id = parsed.get("id").cloned().unwrap_or_else(|| Value::from(0));
    let Some(method) = parsed.get("method").and_then(Value::as_str) else {
        return json!({ "id": id, "err": "missing method" }).to_string();
    };
    let params = parsed.get("params").cloned().unwrap_or(Value::Null);
    let method = method.to_owned();

    // DB-backed calls block; keep them off the async executor (each blocking
    // thread resolves the same primary DB URL).
    match tokio::task::spawn_blocking(move || methods::dispatch(&method, params)).await {
        Ok(Ok(value)) => json!({ "id": id, "ok": value }).to_string(),
        Ok(Err(err)) => json!({ "id": id, "err": err }).to_string(),
        Err(e) => json!({ "id": id, "err": format!("handler panicked: {e}") }).to_string(),
    }
}

// --- HTTP routes ---

/// Liveness/handshake for dev tooling.
async fn status_handler() -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "p2p-chat-web",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(serde::Deserialize)]
struct UploadQuery {
    /// Original filename the browser picked (drives name + MIME detection).
    #[serde(default)]
    name: String,
}

/// Store uploaded attachment bytes content-addressedly (files lane §12) and
/// return the descriptor the client attaches to its outgoing message.
async fn upload_handler(
    Query(query): Query<UploadQuery>,
    body: Bytes,
) -> Result<Json<Value>, (StatusCode, String)> {
    let name = query.name;
    let stored =
        tokio::task::spawn_blocking(move || crate::files::store_outgoing_bytes(&name, &body))
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("upload failed: {e}"),
                )
            })?;
    let attachment = stored.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({ "attachment": attachment })))
}

/// Download a stored attachment blob (browser "open attachment" = fetch this
/// URL; the Flutter client renders it as a link/download).
async fn download_handler(Path(file_id): Path<String>) -> Response {
    let located = tokio::task::spawn_blocking(move || {
        let path = crate::files::file_path(&file_id)?;
        let meta = crate::files::serve_file_meta(&file_id);
        Some((path, meta))
    })
    .await
    .ok()
    .flatten();

    let Some((path, meta)) = located else {
        return (StatusCode::NOT_FOUND, "unknown file").into_response();
    };
    let Ok(bytes) = tokio::fs::read(&path).await else {
        return (StatusCode::NOT_FOUND, "blob missing").into_response();
    };

    let (name, _size, mime) = match meta {
        Some((name, size, mime)) => (name, size, mime),
        None => ("attachment".to_owned(), 0, None),
    };
    let mut headers = HeaderMap::new();
    let content_type = mime.unwrap_or_else(|| "application/octet-stream".to_owned());
    if let Ok(value) = header::HeaderValue::from_str(&content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    if let Ok(value) = header::HeaderValue::from_str(&format!(
        "attachment; filename=\"{}\"",
        disposition_filename(&name)
    )) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    (headers, bytes).into_response()
}

/// Reduce a filename to characters safe inside a quoted header value
/// (quotes/backslashes stripped, control characters replaced).
fn disposition_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '"' | '\\' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;
    use std::future::IntoFuture as _;
    use tokio_tungstenite::tungstenite::Message as WsMessage;
    use tower::ServiceExt as _;

    // --- helpers ---

    fn test_state(token: Option<&str>) -> (WebState, tokio::sync::broadcast::Sender<Value>) {
        let (events, _) = tokio::sync::broadcast::channel::<Value>(EVENT_CHANNEL_CAPACITY);
        let state = WebState {
            events: events.clone(),
            token: token.map(str::to_owned),
        };
        (state, events)
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
        let response =
            frame(r#"{"id": 3, "method": "parseLastSeenMs", "params": {"lastSeen": "1970-01-01 00:00:10"}}"#)
                .await;
        assert_eq!(response, json!({"id": 3, "ok": 10_000}));
        let response =
            frame(r#"{"id": 4, "method": "parseLastSeenMs", "params": {"lastSeen": "not-a-date"}}"#)
                .await;
        assert_eq!(response, json!({"id": 4, "ok": 0}));

        // Network-size buckets (the label rule lives in Rust only).
        let response =
            frame(r#"{"id": 5, "method": "networkSizeLabel", "params": {"peerCount": 2}}"#).await;
        assert_eq!(response, json!({"id": 5, "ok": "Small"}));
        let response =
            frame(r#"{"id": 6, "method": "networkSizeLabel", "params": {"peerCount": 40}}"#).await;
        assert_eq!(response, json!({"id": 6, "ok": "Large"}));

        // Nickname validation (canonical Rust rule).
        let response =
            frame(r#"{"id": 7, "method": "validateNickname", "params": {"name": "alice-1"}}"#).await;
        assert_eq!(response, json!({"id": 7, "ok": true}));
        let response =
            frame(r#"{"id": 8, "method": "validateNickname", "params": {"name": ""}}"#).await;
        assert_eq!(response, json!({"id": 8, "ok": false}));
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
            "saveOutgoingBroadcast",
            "saveOutgoingDm",
            "saveIncomingMessage",
            "createGroup",
            "loadGroupMessages",
            "saveOutgoingGroup",
            "acceptGroupInvite",
            "setSelfNickname",
            "validateNickname",
            "networkSizeLabel",
            "provisionMyUser",
            "sendDeviceBindingIntro",
            "acceptDeviceBinding",
            "verifyDeviceBindingClaim",
            "addDevice",
            "getUserDevices",
            "addContact",
            "removeContact",
            "sortContacts",
            "startCall",
            "sendCallSignal",
            "formatTimeHhmm",
            "isAtBottom",
            "parseLastSeenMs",
        ] {
            let err = methods::dispatch(method, Value::Null)
                .expect_err("params-less call is a parse error");
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
            "getContacts",
            "getContactsWithStats",
            "getPeersWithStats",
            "getMyUserId",
        ] {
            assert!(
                source.contains(&format!("\"{method}\"")),
                "{method} missing from the dispatch table"
            );
        }
    }

    // --- auth ---

    #[test]
    fn no_configured_token_means_open() {
        let (state, _events) = test_state(None);
        let request = http_request("GET", "/api/status", Vec::new());
        assert!(auth_allows(&state, &request));
    }

    #[test]
    fn token_must_match_via_bearer_header_or_query() {
        let (state, _events) = test_state(Some("s3cret"));

        let request = http_request("GET", "/api/status", Vec::new());
        assert!(
            !auth_allows(&state, &request),
            "no credentials must be denied"
        );

        let mut request = http_request("GET", "/api/status", Vec::new());
        request.headers_mut().insert(
            header::AUTHORIZATION,
            "Bearer s3cret".parse().expect("header value"),
        );
        assert!(auth_allows(&state, &request));

        let mut request = http_request("GET", "/api/status", Vec::new());
        request.headers_mut().insert(
            header::AUTHORIZATION,
            "Bearer wrong".parse().expect("header value"),
        );
        assert!(!auth_allows(&state, &request));

        // The WebSocket upgrade carries its token as a query parameter.
        assert!(auth_allows(
            &state,
            &http_request("GET", "/ws?token=s3cret", Vec::new())
        ));
        assert!(!auth_allows(
            &state,
            &http_request("GET", "/ws?token=nope", Vec::new())
        ));
    }

    #[test]
    fn bearer_parsing_needs_the_exact_prefix() {
        let mut headers = HeaderMap::new();
        assert_eq!(bearer_token(&headers), None);
        headers.insert(header::AUTHORIZATION, "s3cret".parse().expect("value"));
        assert_eq!(
            bearer_token(&headers),
            None,
            "a bare token is not a bearer token"
        );
        headers.insert(header::AUTHORIZATION, "Bearer s3cret".parse().expect("value"));
        assert_eq!(bearer_token(&headers).as_deref(), Some("s3cret"));
    }

    #[test]
    fn query_token_parsing_is_exact() {
        assert_eq!(query_token(None), None);
        assert_eq!(
            query_token(Some("a=1&token=abc&b=2")).as_deref(),
            Some("abc")
        );
        assert_eq!(query_token(Some("notoken=abc")).as_deref(), None);
        assert_eq!(query_token(Some("token=")).as_deref(), Some(""));
    }

    // --- HTTP routes ---

    #[tokio::test]
    async fn status_is_open_without_a_token_and_authenticated_with_one() {
        let (state, _events) = test_state(None);
        let response = build_router(state, None)
            .oneshot(http_request("GET", "/api/status", Vec::new()))
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["ok"], true);
        assert_eq!(body["service"], "p2p-chat-web");

        let (state, _events) = test_state(Some("tok"));
        let denied = build_router(state.clone(), None)
            .oneshot(http_request("GET", "/api/status", Vec::new()))
            .await
            .expect("router responds");
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

        let mut authorized = http_request("GET", "/api/status", Vec::new());
        authorized.headers_mut().insert(
            header::AUTHORIZATION,
            "Bearer tok".parse().expect("value"),
        );
        let allowed = build_router(state.clone(), None)
            .oneshot(authorized)
            .await
            .expect("router responds");
        assert_eq!(allowed.status(), StatusCode::OK);

        let allowed = build_router(state, None)
            .oneshot(http_request("GET", "/api/status?token=tok", Vec::new()))
            .await
            .expect("router responds");
        assert_eq!(allowed.status(), StatusCode::OK);
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
        assert_eq!(disposition_filename("ev\"il\\name\n.txt"), "ev_il_name_.txt");
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
            match socket.next().await.expect("socket frame").expect("frame ok") {
                WsMessage::Text(text) => return text.to_string(),
                WsMessage::Close(_) => panic!("server closed the socket"),
                _ => {}
            }
        }
    }

    #[tokio::test]
    async fn websocket_answers_requests_and_pushes_events() {
        let (state, events) = test_state(Some("wstok"));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(
            axum::serve(listener, build_router(state, None)).into_future(),
        );

        let (mut socket, _) =
            tokio_tungstenite::connect_async(format!("ws://{addr}/ws?token=wstok"))
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
    async fn websocket_upgrade_requires_the_token() {
        let (state, _events) = test_state(Some("wstok"));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(
            axum::serve(listener, build_router(state, None)).into_future(),
        );

        assert!(
            tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
                .await
                .is_err(),
            "a token-guarded upgrade must not succeed without the token"
        );

        server.abort();
    }
}
