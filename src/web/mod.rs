//! Web frontend server (`docs/web_frontend.md`).
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
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router, extract::Query};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD as BASE64, URL_SAFE_NO_PAD as BASE64_URL};
use futures_util::{SinkExt as _, StreamExt as _};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use subtle::ConstantTimeEq as _;
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

/// Single-user HTTP Basic credentials guarding the web frontend.
///
/// The same secret keys the browser session cookie (see [`auth_middleware`]),
/// so rotating the password invalidates every existing session.
#[derive(Clone, Debug)]
pub struct Credentials {
    /// Username the browser's credential dialog must present. Cannot contain
    /// `:` (RFC 7617 splits the decoded pair on the first colon).
    pub user: String,
    /// Shared secret. One install = one user, so there is no user table.
    pub password: String,
}

/// Server configuration for [`serve`].
pub struct WebConfig {
    /// Address to bind (`127.0.0.1:8080` by default at the call site).
    pub bind: SocketAddr,
    /// Optional HTTP Basic credentials guarding `/ws`, `/api/*`, and the
    /// static SPA. Required in practice for non-loopback binds; loopback-only
    /// deployments may omit it.
    pub credentials: Option<Credentials>,
    /// Optional directory with the `flutter build web` output to serve as
    /// static assets (SPA fallback to `index.html`).
    pub assets: Option<PathBuf>,
}

/// Shared server state (event fan-out + auth credentials).
#[derive(Clone)]
struct WebState {
    events: tokio::sync::broadcast::Sender<Value>,
    credentials: Option<Credentials>,
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
        credentials: config.credentials.clone(),
    };
    let app = build_router(state, config.assets.as_ref());

    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .map_err(|e| format!("failed to bind {}: {e}", config.bind))?;
    crate::logging::p2plog_info(format!(
        "web frontend listening on http://{}{}{}",
        config.bind,
        if config.credentials.is_some() {
            " (HTTP Basic auth enabled)"
        } else {
            ""
        },
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
        .layer(DefaultBodyLimit::max(body_limit));

    if let Some(assets) = assets {
        let index = assets.join("index.html");
        app = app.fallback_service(ServeDir::new(assets).fallback(ServeFile::new(index)));
    }
    // Auth is installed AFTER the fallback so the static SPA is gated too: a
    // browser must log in before it can even load the app shell. CORS stays
    // outermost so preflight `OPTIONS` (which never carries credentials) is
    // answered instead of 401'd.
    app.layer(middleware::from_fn_with_state(
        state.clone(),
        auth_middleware,
    ))
    .layer(CorsLayer::permissive())
    // Convert `Router<WebState>` into a servable `Router<()>` (the state is
    // already attached to the middleware; this fills the fallback slot).
    .with_state(state)
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

/// Cookie carrying the browser session minted by a successful HTTP Basic
/// login. Browsers cannot set headers on a `WebSocket` handshake, so the
/// session cookie is how `/ws` authenticates.
const SESSION_COOKIE: &str = "p2p_web_session";
/// Browser session lifetime in seconds (7 days); a fresh login slides it.
const SESSION_TTL_SECS: i64 = 604_800;
/// Domain the session MAC is bound to (see [`session_mac`]).
const SESSION_MAC_DOMAIN: &[u8] = b"p2p-web-session";
/// Realm shown by the browser's credential dialog.
const AUTH_CHALLENGE: &str = "Basic realm=\"p2p_app\", charset=\"UTF-8\"";

/// HTTP Basic gate for `/ws`, `/api/*`, and the static SPA.
///
/// No credentials configured → open (the loopback default). Otherwise a
/// request is allowed when it carries a valid session cookie or a valid
/// `Authorization: Basic` header; the latter also mints the cookie so the
/// browser's later requests (and the `WebSocket` handshake) need no header.
async fn auth_middleware(State(state): State<WebState>, request: Request, next: Next) -> Response {
    let Some(credentials) = state.credentials.as_ref() else {
        return next.run(request).await;
    };
    if session_cookie_is_valid(credentials, request.headers()) {
        return next.run(request).await;
    }
    if !basic_credentials_match(credentials, request.headers()) {
        return unauthorized();
    }
    let mut response = next.run(request).await;
    if let Ok(cookie) = HeaderValue::from_str(&session_cookie(credentials)) {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    response
}

/// `401` plus the challenge that makes browsers show their credential dialog.
fn unauthorized() -> Response {
    let mut response = (StatusCode::UNAUTHORIZED, "authentication required").into_response();
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static(AUTH_CHALLENGE),
    );
    response
}

/// `Authorization: Basic base64(user:password)` decoded to `user:password`.
fn basic_credentials(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let encoded = value.strip_prefix("Basic ")?;
    let decoded = BASE64.decode(encoded.trim()).ok()?;
    String::from_utf8(decoded).ok()
}

/// Compare the presented pair with the configured one in constant time, so
/// the password cannot be recovered by measuring how early a mismatch is.
fn basic_credentials_match(credentials: &Credentials, headers: &HeaderMap) -> bool {
    let Some(presented) = basic_credentials(headers) else {
        return false;
    };
    let expected = format!("{}:{}", credentials.user, credentials.password);
    expected.as_bytes().ct_eq(presented.as_bytes()).into()
}

/// `Set-Cookie` value for a fresh session: `expiry.MAC`, both derived from the
/// credentials, so sessions are stateless (no store to prune, and rotating the
/// password invalidates them all).
fn session_cookie(credentials: &Credentials) -> String {
    format!(
        "{SESSION_COOKIE}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={SESSION_TTL_SECS}",
        session_cookie_value(credentials)
    )
}

/// The `expiry.MAC` part of [`session_cookie`] (also what a client sends back
/// in its `Cookie` header).
fn session_cookie_value(credentials: &Credentials) -> String {
    let expires_at = chrono::Utc::now()
        .timestamp()
        .saturating_add(SESSION_TTL_SECS);
    let mac = BASE64_URL.encode(session_mac(credentials, expires_at));
    format!("{expires_at}.{mac}")
}

/// Keyed digest of `domain ‖ user ‖ password ‖ expiry`. The password is the
/// key, so an attacker cannot forge a cookie without knowing it.
///
/// A plain keyed hash (not HMAC) is enough here: the message ends in a
/// canonical integer, and verification re-derives it from the *parsed*
/// `expiry`, so a length-extension MAC could never be paired with an expiry
/// that parses (and the digest itself is only ever shown to its holder).
fn session_mac(credentials: &Credentials, expires_at: i64) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(SESSION_MAC_DOMAIN);
    hasher.update(b"\0");
    hasher.update(credentials.user.as_bytes());
    hasher.update(b"\0");
    hasher.update(credentials.password.as_bytes());
    hasher.update(b"\0");
    hasher.update(expires_at.to_string().as_bytes());
    hasher.finalize().to_vec()
}

fn session_cookie_is_valid(credentials: &Credentials, headers: &HeaderMap) -> bool {
    let Some(value) = cookie_value(headers, SESSION_COOKIE) else {
        return false;
    };
    let Some((expires, mac)) = value.split_once('.') else {
        return false;
    };
    let Ok(expires_at) = expires.parse::<i64>() else {
        return false;
    };
    if chrono::Utc::now().timestamp() >= expires_at {
        return false;
    }
    let Ok(presented) = BASE64_URL.decode(mac) else {
        return false;
    };
    session_mac(credentials, expires_at)
        .as_slice()
        .ct_eq(&presented)
        .into()
}

/// Value of `name` from any `Cookie` header (`a=1; name=value; b=2`).
fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .find_map(|pair| {
            let (key, value) = pair.trim().split_once('=')?;
            (key == name).then(|| value.to_owned())
        })
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
        let response = frame(
            r#"{"id": 1, "method": "formatTimeHhmm", "params": {"dt": "2026-10-19 08:42:00"}}"#,
        )
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
        let response = frame(
            r#"{"id": 4, "method": "parseLastSeenMs", "params": {"lastSeen": "not-a-date"}}"#,
        )
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
            frame(r#"{"id": 7, "method": "validateNickname", "params": {"name": "alice-1"}}"#)
                .await;
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
            "deleteGroup",
            "setSelfNickname",
            "setPeerLocalNickname",
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
                let reuploaded_id =
                    crate::files::store_outgoing_bytes("other-name.txt", &checks_data)
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
            tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(
                format!("ws://{addr}/ws"),
            )
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
            tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(
                format!("ws://{addr}/ws"),
            )
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
}
