# Web frontend (2026-10-18) — Flutter Web as a display client over a node server

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

- Default bind `127.0.0.1` — no token required, same trust model as the TUI.
- `--bind 0.0.0.0 --token <t>` for LAN exposure: token enforced on `/ws` and `/api/*`
  (`Authorization: Bearer` or `?token=`). The Flutter web client reads `?token=` from
  `Uri.base` at startup and attaches it to every call. Static assets stay open.

## Rust touch points

- `Cargo.toml`: `web = ["dep:axum", "dep:tower-http"]` (added to `default`),
  `[[bin]] p2p_chat_server` (`required-features = ["web"]`).
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

Verified locally: `cargo ct`, `cargo test --all-features` (1166), `flutter analyze`,
`flutter test` (69), `flutter build web` (`✓ Built build/web`), and a manual smoke run of the
binary (`/api/status`, SPA index + `main.dart.js`, token enforced via both `Authorization:
Bearer` and `?token=`, static assets open).

`flutter build web` prints a `Wasm dry run failed … invalid_runtime_check_with_js_interop_types`
warning from **flutter_rust_bridge 2.13.0 itself** (not this app's code); the build then
compiles to JS normally. Pass `--no-wasm-dry-run` to silence it.

## Building and running

```bash
scripts/build-web.sh            # release; add --debug for a fast local build
./target/release/p2p_chat_server --assets apps/flutter_app/build/web
# LAN exposure (token required off loopback):
./target/release/p2p_chat_server --bind 0.0.0.0 --token <secret> --assets apps/flutter_app/build/web
```

## Out of scope

- Compiling the Rust core to WASM (browser-as-node); revisit only if a fully self-contained
  browser node is ever wanted.
- Call media (signaling only, same as every other frontend).
