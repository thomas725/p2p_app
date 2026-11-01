# Dependencies

Every direct dependency, why it is here, and where it is pinned.

**Regenerate from the manifest:** the table below is derived from
`Cargo.toml` + `Cargo.lock`. `Cargo.lock` is the authority on the *resolved*
version; `Cargo.toml` is the authority on the *requirement*.

## Networking and protocol

| Crate | Requirement | Resolved | Why it is here |
|---|---|---|---|
| `libp2p` | `=0.57.1` | 0.57.1 | The whole peer-to-peer layer: transports, muxing, discovery, and the four `request_response` protocols. Features: `tokio`, `gossipsub`, `noise`, `macros`, `tcp`, `yamux`, `quic`, `request-response`, `serde`, `json`, `cbor`, `ping`. Optional — behind `basic` (and `mdns`/`quic`). |
| `libp2p-identity` | `=0.3.0` | 0.3.0 | Keypair generation and peer-id derivation. Pinned to keep a single copy of `libp2p-identity` in the tree (a second copy would mean two incompatible `PeerId` types). |
| `tokio` | `1.53.1` | 1.54.1 | The async runtime libp2p runs on. Features: `macros`, `rt-multi-thread`, `io-util`, `io-std`. 1.54.1 also carries the `AsyncFd` I/O-safety fix (RUSTSEC-2026-0386, patched from 1.53.3). |
| `axum` | `0.8` | 0.8.9 | The web frontend's HTTP + WebSocket server (`src/web/`). Features: `ws`, `json`, `query`, `tokio`, `http1`, `macros`. Optional — behind `web`. |
| `tower-http` | `0.7` | 0.7.1 | Static-file serving and CORS for the web server (`ServeDir`/`ServeFile`, `CorsLayer`). Features: `cors`, `fs`. Optional — behind `web`. |
| `futures-util` | `0.3` | 0.3.34 | `Stream`/`Sink` combinators, used to split the axum WebSocket. Optional — behind `web`. |
| `base64` | `0.22` | 0.22.1 | RFC 7617 `Authorization: Basic` decoding and the session-cookie MAC encoding for the web frontend's auth. Optional — behind `web`. Already in the tree via libp2p (0.22.1 and 0.23.1), so this adds no new crate. |
| `subtle` | `2` | 2.6.1 | Constant-time comparison of the Basic password and the session-cookie MAC, so auth cannot be probed byte-by-byte by timing. Optional — behind `web`. Already in the tree. |

> The old `futures` umbrella crate was replaced by `futures-util` — the
> project only needs the stream/sink traits, not the executor.

## Persistence

| Crate | Requirement | Resolved | Why it is here |
|---|---|---|---|
| `diesel` | `^2.3` | 2.3.14 | The ORM and query builder for SQLite. Features: `sqlite`, `returning_clauses_for_sqlite_3_35`, `chrono`. 2.3.14 is the first 2.x that accepts `libsqlite3-sys` 0.38 (it requires `>=0.17.2, <0.39.0`). |
| `diesel_migrations` | `^2.3` | 2.3.2 | Embeds and runs the SQL migrations in `migrations/` at startup. |
| `libsqlite3-sys` | `0.38` | 0.38.2 | Native SQLite FFI. Feature `bundled` compiles SQLite in, which Android needs. Optional — behind `sqlite_bundled`. The `0.38` requirement is diesel 2.3.14's ceiling; 0.39 is still outside its range. |
| `chrono` | `0.4.45` | 0.4.45 | Timestamps for the `TIMESTAMP` columns. Feature `clock`. |

## Serialization

| Crate | Requirement | Resolved | Why it is here |
|---|---|---|---|
| `serde` | `1.0` | 1.0.229 | Derives for every wire and row type. Feature `derive`. |
| `serde_json` | `1.0` | 1.0.154 | The JSON codec for the direct-message lane and the web server's request bodies. |
| `sha2` | `0.11` | 0.11.1 | SHA-256 for content-addressed attachments: an attachment's `file_id` is the hex digest of its bytes. The lock also keeps `sha2 0.10.9` for `snow → libp2p-noise`, whose requirement has not moved. |

## Frontends

| Crate | Requirement | Resolved | Why it is here |
|---|---|---|---|
| `ratatui` | `=0.30.2` | 0.30.2 | The TUI's rendering library. Optional — behind `tui`. |
| `ratatui-textarea` | `0.9` | 0.9.3 | The multi-line text input widget. Optional — behind `tui`. |
| `crossterm` | `=0.29` | 0.29.0 | Terminal raw mode, key events, and mouse capture. Optional — behind `tui`. |
| `flutter_rust_bridge` | `=2.13.0` | 2.13.0 | The FFI bridge for the Flutter app. Pinned exactly: FRB's codegen and the Rust crate must agree on the version. Optional — behind `mobile`. |

## Naming, formatting, and crypto helpers

| Crate | Requirement | Resolved | Why it is here |
|---|---|---|---|
| `petname` | `3.0` | 3.2.0 | Generates the fallback display names for peers with no nickname. |
| `unicode-width` | `0.2` | 0.2.2 | Display-width measurement, so table cells pad correctly for wide characters. |
| `num-traits` | `0.2` | 0.2.19 | `ToPrimitive` for safe `f64 → i64` timestamp truncation — the project denies `as` conversions. |
| `tracing` | `=0.1.44` | 0.1.44 | Structured logging; libp2p emits through it. Optional — behind `tracing`. |
| `tracing-subscriber` | `=0.3.23` | 0.3.23 | The log formatter/filter. Optional — behind `tracing`. |
| `color-eyre` | `=0.6.5` | 0.6.5 | Error reporting with context and a readable backtrace. |

## Development dependencies

| Crate | Requirement | Resolved | Why it is here |
|---|---|---|---|
| `tempfile` | `3` | 3.27.0 | Scratch databases and directories in tests. |
| `serial_test` | `4` | 4.0.1 | Serializes the tests that touch the shared SQLite file. |
| `tower` | `0.5` | 0.5.3 | `Router::oneshot` for testing the web server's HTTP routes. Feature `util`. |
| `tokio-tungstenite` | `0.29` | 0.29.0 | A WebSocket client for the web server's end-to-end frame-protocol test. Held at 0.29 on purpose: axum's `ws` feature already pulls `tokio-tungstenite 0.29 → tungstenite 0.29`, so moving this test-only client to 0.30 would compile a second tungstenite tree (plus `sha1` 0.11 and `rand` 0.10). Revisit when axum moves. |

## Known dependency state

### `cargo audit` — clean

`cargo audit` exits 0. `.cargo/audit.toml` is **comment-only**: no advisory is
silently ignored. Two rows remain, both allowed *warnings*:

| Crate | Advisory | Why it is acceptable |
|---|---|---|
| `paste` 1.0.15 | RUSTSEC-2024-0436 (unmaintained) | A build-time proc-macro pulled in by `libp2p-mdns → if-watch → netlink-packet-core`. Benign, and it is core libp2p's dependency, not ours. |
| `yoke-derive` 0.8.3 | yanked | A transitive yanked release with no fix published. |

Everything else has been fixed upstream and pulled in. Recent rounds:

- **2026-10-29** — `ctutils 0.4.2 → 0.4.7` (RUSTSEC-2026-0385: non-constant-time
  mask generation and `u128` comparisons; patched from 0.4.4) and
  `tokio 1.53.1 → 1.54.1` (RUSTSEC-2026-0386: `AsyncFd` constructors violate I/O
  safety; patched from 1.53.3). `ctutils` is transitive
  (`digest 0.11 → curve25519-dalek → ed25519-dalek → libp2p-identity`), so it
  moves only through the lockfile.
- **2026-10-21** — **rustls 0.23.45 → 0.23.46** and
  **rustls-webpki 0.103.15 → 0.103.16**, clearing five advisories
  (RUSTSEC-2026-0346/0347/0348/0349 and RUSTSEC-2026-0382).

### `cargo outdated --depth 1` — two direct rows, both held

The 2026-10-29 → 11-01 dependency round cleared every lockfile-only row
(`diesel 2.3.14`, `ratatui-textarea 0.9.3`, `serde_json 1.0.154`,
`tokio 1.54.1`), deliberately moved the pinned `libp2p` from 0.57.0 to 0.57.1
(a patch release that pulled no subcrate version changes, so the protocol code
is untouched), took `tower-http 0.6.11 → 0.7.1` (no code change —
`ServeDir`/`ServeFile`/`CorsLayer` kept their shape — and no new transitives),
and finally moved `sha2 0.10 → 0.11.1`.

That `sha2` bump needed **no source change**: `hash_bytes` in `src/files.rs`
still compiles and the hex digest is byte-identical, which the existing
`files::tests::hash_bytes_is_lowercase_hex_sha256` test pins down. The tree now
carries two `sha2` lines on purpose: our direct `0.11.1` and `0.10.9` under
`snow → libp2p-noise`, whose upstream range has not moved.

Nothing actionable is left. Both remaining rows are **held on purpose** — do
not "fix" these without revisiting the reason:

| Crate | Locked | Newer | Why it is held |
|---|---|---|---|
| `libsqlite3-sys` | 0.38.2 | 0.39.1 | Diesel 2.3.14 accepts `>=0.17.2, <0.39.0`, so 0.38.2 is as far as the tree can go until a diesel release widens the range. Do not restructure to `sqlite-bundled` — that was evaluated and rejected; see [backlog.md](backlog.md). |
| `tokio-tungstenite` | 0.29.0 | 0.30.0 | Test-only client, kept in lockstep with axum's `ws` feature. Bumping it to 0.30 adds a *second* `tungstenite` tree (`0.29` from axum, `0.30` here, plus `sha1` 0.11 and `rand` 0.10) for no behavioural gain — verified by building the bump and backing it out. Revisit when axum moves. |

The full `cargo outdated` output also lists transitive rows. Those are not
actionable here — they move when their parent moves.

### `cargo audit` is the only hard dependency gate

The dependency workflow (`dependencies.yml`) fails on any advisory that is
not ignored in `.cargo/audit.toml`. `cargo outdated` is **informational**:
it exits non-zero whenever any row exists, including transitive pins no
repo change can move, so it must not gate the workflow.

## Conventions

- **Pin what must not drift.** `libp2p`, `libp2p-identity`, `flutter_rust_bridge`,
  `ratatui`, `crossterm`, `tracing`, `tracing-subscriber`, and `color-eyre`
  use `=` requirements: a silent minor bump in any of them changes the
  protocol or the FFI contract.
- **Range what is safe to range.** `diesel`, `serde`, `chrono`, `axum`,
  `tokio`, and the rest use caret requirements.
- **A new dependency needs a row in this file** — say what it is for, and
  whether it is behind a feature flag.
