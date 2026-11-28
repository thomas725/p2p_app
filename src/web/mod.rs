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
pub(crate) mod params;

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
/// The same secret keys the browser session cookie, so rotating the password
/// invalidates every existing session.
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
    // `subtle`'s slice `ct_eq` short-circuits when the lengths differ, which
    // would leak the configured secret's length through response timing.
    // Hashing both sides to fixed 32-byte digests first makes the comparison
    // branch only on content, honoring the constant-time guarantee.
    let expected_hash = Sha256::digest(expected.as_bytes());
    let presented_hash = Sha256::digest(presented.as_bytes());
    expected_hash
        .as_slice()
        .ct_eq(presented_hash.as_slice())
        .into()
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

async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<WebState>,
) -> Response {
    // Browsers cache HTTP Basic credentials per origin and attach them to WS
    // handshakes regardless of the session cookie's `SameSite` policy, so with
    // auth enabled a page the user visited after logging in could otherwise
    // drive the whole API through a cross-site socket (CSWSH). Every browser
    // WS handshake carries `Origin`; non-browser clients do not, so only a
    // present-but-mismatched Origin is rejected.
    if state.credentials.is_some() && !ws_origin_matches_host(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Whether a WS handshake's `Origin` (when present) matches the request's
/// `Host`. Absent `Origin` is allowed so native/test WS clients keep working;
/// a present one must carry the Host as its `scheme://authority`.
fn ws_origin_matches_host(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return true;
    };
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    origin
        .split_once("://")
        .is_some_and(|(_, authority)| authority.eq_ignore_ascii_case(host))
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
#[path = "../../tests/unit/unit_web_mod.rs"]
mod tests;
