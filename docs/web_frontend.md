# Web frontend — Flutter Web as a display client over a node server

**Status: shipped.** This document records the decision and the wire
protocol. The code (`src/web/`, `apps/flutter_app/lib/src/data/http_node_api.dart`)
is the authority for current behaviour.

## Decision

The web frontend keeps the **node native on the server machine** and makes the browser a
pure display client:

- **Rust**: new `web` cargo feature (in `default`) adding an **axum** HTTP+WebSocket server
  (`src/web/`) that mirrors the whole FRB surface (`src/api/mod.rs` + the `mobile_api`
  helpers) as JSON methods over one WebSocket. Attachment bytes move over plain HTTP
  (upload `POST /api/upload`, download `GET /api/files/{file_id}`). The Flutter web build is
  served as static assets from the same origin. New binary `p2p_chat_server` starts the node
  exactly like the mobile path (`mobile_node::start_node_auto`: provisioning, group
  re-subscribe, GC sweep) and then serves.
- **Flutter**: screens keep their generated model types but call through a new `NodeApi`
  abstraction (`lib/src/data/node_api.dart`): `FrbNodeApi` (in-process FRB — unchanged
  Android behavior) and `HttpNodeApi` (WS/HTTP JSON — used when `kIsWeb`). Same screens,
  same Rust sorting/validation, zero WASM compilation of the core.

This was chosen over (a) a full WASM node in the browser and (b) naive Flutter-Web-with-FRB
because both require porting libp2p TCP/QUIC, mDNS, diesel/SQLite and the disk blob store
to WASM — a multi-week effort losing features — while the server-client split keeps 100 %
feature parity with TUI and mobile from day one.

## Wire protocol (WebSocket `/ws`)

- Request:  `{"id": <int>, "method": "<name>", "params": <any>}`
- Response: `{"id": <int>, "ok": <result>}` or `{"id": <int>, "err": "<message>"}`
- Push:     `{"event": <SwarmEventJson>}` (server polls the node's event queue at 50 ms and
  pushes; replaces Flutter's 200 ms `pollEvent` loop on web)

Method names mirror the FRB function names 1:1 (`loadDmMessages`, `saveOutgoingDm`,
`getContactsWithStats`, …) so the Dart `HttpNodeApi` is a mechanical mapping.

## HTTP routes

- `POST /api/upload?name=<filename>` — raw bytes body → stored content-addressed →
  `{"attachment": MessageAttachment}`.
- `GET /api/files/{file_id}` — blob download (Content-Type from name, attachment filename).
- `GET /api/status` — liveness for dev tooling.
- Static SPA: `--assets <dir>` serves the `flutter build web` output with index.html
  fallback for client-side routes.

## Security model

Single-user, shared-secret, browser-native: **HTTP Basic auth** (`RFC 7617`).

- Default bind `127.0.0.1` — no credentials required, same trust model as the TUI.
- Set `--password <secret>` (or, preferably, `P2P_WEB_PASSWORD`) to enable auth.
  The username is `p2p` unless `--user` / `P2P_WEB_USER` overrides it.
- The gate covers **everything**: `/ws`, `/api/*`, *and* the static SPA. A
  browser is challenged (`401` + `WWW-Authenticate: Basic realm="p2p_app"`)
  before it can load a single asset, and the browser's own credential dialog —
  not an app-provided login form — collects the secret.
- A successful login mints the `p2p_web_session` cookie
  (`HttpOnly; SameSite=Strict; Path=/`, 7-day `Max-Age`, refreshed on each
  login). It exists because **JavaScript cannot set headers on a `WebSocket`
  handshake**: the cookie rides the `/ws` upgrade and every attachment route,
  so the SPA never sees the password and never puts it in a URL.
- The cookie value is `expiry.MAC`, where the MAC is
  `SHA-256("p2p-web-session" ‖ user ‖ password ‖ expiry)`. Sessions are
  therefore **stateless** (nothing to store or prune) and **rotating the
  password invalidates every session**. Comparisons are constant-time.
- Secrets never appear in a URL: the old `?token=` query parameter is gone, so
  nothing lands in browser history, `Referer` headers, or proxy access logs.
- **There is no TLS of its own.** Basic credentials are base64 (not encrypted),
  so on an untrusted LAN put the server behind a TLS-terminating reverse proxy
  and bind it to loopback. `Secure` is deliberately not set on the cookie —
  that would break plain-HTTP LAN use.

Programmatic clients (curl, scripts, health checks) use the same credentials:

```bash
curl -u p2p:"$P2P_WEB_PASSWORD" http://127.0.0.1:8080/api/status
```

## Rust touch points

- `Cargo.toml`: `web = ["dep:axum", "dep:tower-http", "dep:base64", "dep:subtle"]` (added to
  `default`), `[[bin]] p2p_chat_server` (`required-features = ["web"]`). `base64` and
  `subtle` were already in the tree; they are the Basic decoding and the constant-time
  comparison for the auth gate.
- Wire structs gain `Serialize` (+ `Deserialize` for sort inputs) — no FRB impact
  (`rust_input` is `api,mobile_api,mobile_node`; new fns stay `pub(crate)`).
- `files::store_outgoing_bytes` (upload handler), `mobile_node::save_outgoing_*_with_attachment`
  (path-based variants delegate to these).

## Flutter touch points

- `lib/src/data/node_api.dart` (interface), `frb_node_api.dart`, `http_node_api.dart`.
- `event_bus.dart` delegates event pumping to `NodeApi.startEventPump`.
- `attachments.dart`: web pick → HTTP upload; web open → browser download URL.
- `main.dart`/`home.dart`: `kIsWeb` boot branch (no FRB init, connect `HttpNodeApi` to
  same-origin `/ws`).
- New deps: `web_socket_channel`, `http`, `url_launcher`.

## Status

Implemented and verified end to end:

- Rust: `web` feature + `src/web/{mod,methods}.rs` (WS dispatch + HTTP routes), the
  `p2p_chat_server` binary, and unit tests inside `src/web/mod.rs` / `src/api/mod.rs`.
- Flutter: `NodeApi` abstraction with `FrbNodeApi`/`HttpNodeApi`/`FakeNodeApi`, the `kIsWeb`
  boot branch, web attachment pick/open, plus Dart transport tests.
- Every FRB-exposed method has a matching server arm; the no-param arms are guarded by the
  source-string drift list in `src/web/mod.rs`, and every parameterised arm is covered by a
  `methods::dispatch(name, Null)` routing test asserting `invalid params:`.
- Feature surface equals the union of the TUI and Flutter feature sets (group leave, per-peer
  local nickname, calls, device binding, files/attachments, backfill, stats).

Verified by the in-module tests in `src/web/mod.rs`: frame protocol, every routed
method, the HTTP routes, HTTP Basic (denied / wrong pair / accepted + `Set-Cookie`),
session-cookie round trip with forged, expired and garbage values rejected, static-asset
gating, and a real `tokio-tungstenite` WebSocket upgrade authenticated by the session
cookie (and refused without one).

`flutter build web` prints a `Wasm dry run failed … invalid_runtime_check_with_js_interop_types`
warning from **flutter_rust_bridge 2.13.0 itself** (not this app's code); the build then
compiles to JS normally. Pass `--no-wasm-dry-run` to silence it.

## Building and running

```bash
scripts/build-all.sh              # TUI + web frontend in one go (release; --debug for a fast build)
scripts/build-web.sh              # web frontend only
./target/release/p2p_chat_tui     # the terminal frontend
./target/release/p2p_chat_server --assets apps/flutter_app/build/web
# LAN exposure (auth required off loopback): the browser prompts for these
export P2P_WEB_PASSWORD='<secret>'   # P2P_WEB_USER defaults to `p2p`
./target/release/p2p_chat_server --bind 0.0.0.0 --assets apps/flutter_app/build/web
```

`scripts/build-all.sh` builds **both** desktop frontends: the TUI binary and the Flutter web
bundle, from a single `cargo build --bin p2p_chat_tui --bin p2p_chat_server` (one feature
resolution, shared dependency graph) plus `flutter build web`. The node stays native in the Rust
web server in both cases — the browser bundle is a display client only. `--tui-only` /
`--web-only` build one half.

## Out of scope

- Compiling the Rust core to WASM (browser-as-node); revisit only if a fully self-contained
  browser node is ever wanted.
- Call media (signaling only, same as every other frontend).
