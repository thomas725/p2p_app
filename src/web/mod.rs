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

    if let Some(assets) = &config.assets {
        let index = assets.join("index.html");
        app = app.fallback_service(ServeDir::new(assets).fallback(ServeFile::new(index)));
    }
    // Convert `Router<WebState>` into a servable `Router<()>` (the state is
    // already attached to the middleware; this fills the fallback slot).
    let app = app.with_state(state);

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
