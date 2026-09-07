# AGENTS.md — operating rules for this repo

Archived history (goal, done-index, decision rationale, backlog recap, descriptive
context) lives in `docs/2026-09-07_archived_agents_context.md` and the dated docs it
indexes. This file keeps only instruction-type guidance for future tasks.

## Constraints & Preferences
- **Prefer nix tooling for anything not already installed as a system tool**: use `nix-shell` and similar nix tools (e.g. `nix-shell -p python3Packages.pyyaml`) whenever something isn't installed as a system tool. Do **not** reach for `podman`/`docker` for this — only use them if there is no nixpkgs package available for what's needed.
- Toolchain: **nightly** (`rustup override set nightly` in `/home/user/project`).
- All code must be clippy-clean with the project's strict pedantic config: `cargo ct` (alias for `cargo clippy --all-targets --all-features`) must exit 0 with no warnings.
- User decisions: peer click in Peers tab **keeps opening the DM tab**; message click in Chat/Log/DM **opens the sender's/partner's PeerInfo**.
- **Committing**: commit when a logical block of work is done and it makes sense; only **pushing** is reserved for the user.

## Progress
### In Progress
- (none)

### Blocked
- (none)

## Code Rules (do/don't)
- **PeerInfo is a transient dynamic tab** (like DM), not a fixed tab: inserted after DMs, before Log/Settings. Dedup by peer ID.
- **PeerInfo binding is tab-specific**: plain `i` opens PeerInfo only on the **Peers** tab (no text input there). On **Direct/DM** tabs use `Ctrl+I` on kitty terminals (WezTerm) and `Ctrl+P` on every terminal. Bare Tab **always** cycles tabs on every tab and terminal. Shortcut bar only documents a binding where it is active.
- **Display-name memoization keeps the DB-backed resolution path**: don't swap the sort/render to in-memory nickname maps — `get_peer_display_name` caches the exact string it would compute (local → received → generated petname + short-ID suffix), so sort and render always agree. Invalidate exactly where nicknames change: `impl_set_peer_field!` setter macro and `peers::save_peer`; `db::set_db_url`/`reset_db_url` must keep calling `nickname::clear_display_names()`.
- **Warm the display-name cache lazily**, on the first frame that draws the Peers tab (`state.peer_names_warmed` in the render loop's draw closure), not at startup. Warm-at-startup was rejected.
- **The sort must never drop peers**: feed the whole `VecDeque` to `sort_peers_by_column` and rebuild via `HashMap<&str, PeerRecord>` — never `peers.as_slices().0` (a wrapped ring loses its second segment).
- **One canonical last-seen parser**: `fmt::parse_last_seen_ms` (space-or-T `%Y-%m-%dT%H:%M:%S`, 0 on failure) is the single source for both the TUI sort and the FRB/mobile wrapper — Dart never parses its own sort keys.
- **Keep TUI/Flutter sort parity**: Flutter sorts via Rust `mobile_api::sort_peers` (mirrors `tui_helpers::sort_peers_table`, ties on `peer_id`, descending reverses the whole `Ordering`). Don't regress to a Dart `List.sort` closure.
- **`get_known_peers` (api wrapper) is a type anchor, not a live call path**: it is the only FRB-reachable reference to `MobilePeerRecord` (defined in the frb-ignored `mobile_node`). Deleting it makes the type vanish from the generated Dart and breaks `flutter analyze`. Keep it with a doc note pointing to the bulk call.
- **`apps/flutter_app/lib/src/rust/messages.dart` is a hand-maintained orphan** (`PeerMessageStats` isn't in FRB's `rust_input`; regen never touches it). Verify by hand after any change touching the peer-stats surface — including flipping the broadcast field to `broadcastSentToPeer`.
- **`broadcast_recipients` is the single per-peer "broadcast" source**: a broadcast *we* send is recorded once per connected peer (`record_broadcast_recipients`, idempotent), `confirmed_at` back-filled on ack. The "Broadcast" column uniformly means *broadcasts-sent-to-peer* in both frontends.
- **Migration version keys are the date+time, NOT the directory stem**: two migrations sharing a timestamp (`000000`) collide silently. Use distinct suffixes (`...000000_...` / `...000001_...`).
- **Stateful table windowing**: ratatui's `Table` + `TableState` is the "widget that allows scrolling visually" — it keeps the selected row visible. `tui_helpers::peer_table_visible_range` is the pure reimplementation that the click handler and renderer both agree on; keep them consistent.

## Next Steps
- Tracked in **`docs/2026-09-02_backlog.md`**. None urgent. Revisit item 6 (expected `cargo outdated` rows) when libp2p 0.57+ lands (its master unblocker).

## Critical Context
- **FRB regeneration protocol**: `flutter_rust_bridge_codegen generate` (2.13.0, pinned `=2.13.0` in Cargo.toml and `2.13.0` in pubspec) → re-append `clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction` to `src/frb_generated.rs`'s `#![allow(...)]` → `cargo fmt --all` → delete untracked `src/frb_generated.h` → verify `cargo ct` + `cargo test --all-features` + `flutter analyze`/`flutter test`. `pub mod generated;` in `src/lib.rs` carries the `/// flutter_rust_bridge:ignore` doc line that makes FRB skip the diesel schema. `build.rs` regenerates `src/generated/columns.rs` from `schema.rs` at build time.
- **Dependency audit config**: `.cargo/audit.toml` ignores `RUSTSEC-2026-0118`/`RUSTSEC-2026-0119` (hickory-proto 0.25.2 — unfixable while libp2p stays 0.56). CI dependency job gates on `cargo audit` (hard fail) + artifact upload; `cargo outdated` is informational.
- **Scheduled-workflow skip checks need `actions: read`**: `.github/workflows/metrics.yml` and `dependencies.yml` run `scripts/ci-skip-no-new-commits.sh`, which queries the Actions runs API with the GITHUB_TOKEN. The token must carry `actions: read`, or that endpoint returns 403 and (previously, fail-open) every nightly run executed anyway — producing duplicate `ci: refresh coverage and codebase metrics` commits on main. The script is fail-closed now: any API error skips the run (a manual `workflow_dispatch` overrides). Don't drop that permission when editing the workflows.
- `handle_mouse_left_click(state, mouse_row, mouse_column, is_peers_tab)` keeps its 4-arg signature (tests depend on it).

## Relevant Files
- `src/tui_tabs.rs` — `TabContent::PeerInfo`, `peer_info_tabs`, add/remove/titles/content/count.
- `src/mobile_api.rs` — FRB-exposed facade: `format_time_hhmm`, `is_at_bottom`, `calculate_visible_range`, `validate_nickname`, `parse_last_seen_ms` (delegates to `fmt::`), `sort_peers` + `PeerSortInput` (canonical peer-table sort for Dart), and `get_peers_with_stats` + `PeerWithStats` (single-round-trip peers + counts, via `messages::get_all_peer_stats`, replacing Flutter's N+1); `PeerWithStats.broadcast_sent_to_peer` replaces the removed `broadcast_received`. Mobile-gated (not compiled without `feature = "mobile"`).
- `src/messages.rs` — `PeerMessageStats` (`dm_count` + `broadcast_sent_to_peer`) + single-peer `get_peer_stats` (N×2 queries, DM + `broadcast_recipients` counts) and the bulk `get_all_peer_stats` (two passes: one `GROUP BY` with a `UNION ALL` un-nesting DMs against both endpoints, plus a grouped `count(*)` over `broadcast_recipients`), `QueryableByName` `PeerMessageAggregate`; `save_receipt` back-fills `broadcast_recipients.confirmed_at` on kind-0 acks. No longer reads `peers.broadcasts_sent`.
- `src/api/mod.rs` — FRB surface; `get_known_peers` retained as the `MobilePeerRecord` type anchor (Dart's PeerInfoScreen uses the type), `get_peer_stats` wrapper removed (dead after the bulk refactor).
- `apps/flutter_app/lib/main.dart` — `_PeerListState` uses the Rust `sortPeers` helper over `PeerSortInput`; `_refreshPeers` uses the single `getPeersWithStats()` round trip; `_broadcastCount` reads `broadcastSentToPeer` (matched to the TUI).
- `apps/flutter_app/lib/src/rust/messages.dart` — hand-maintained orphan `PeerMessageStats` class; field is `broadcastSentToPeer`. Verify by hand after any stats reform.
- `src/bin/tui/click_handlers.rs` — `handle_message_click`, fixed peer-click guard.
- `src/bin/tui/input_processor.rs` — `open_peer_info_for_active_tab`, `handle_esc_key`, PeerInfo Enter branch, `i` (Peers-only) + Ctrl+I (Direct-only) arms.
- `src/bin/tui/scroll_handlers.rs` — PeerInfo no-op arms, `expected_peer_page_size`, paged `compute_new_peer_selection`, `scroll_peers_tab`.
- `src/bin/tui/render_loop/layout.rs` — `render_shortcuts` (tab-aware) + `shortcuts_text`.
- `src/bin/tui/render_loop/mod.rs` — 5-chunk `render_frame`, sets `chat_area_height` (`height - 10`)/`terminal_width`; renders PeerInfo; `app_state_to_render_state` nickname maps + `peer_table_offset`.
- `src/tui_render.rs` — `render_peer_info_content`, `render_peers_content` (stateful `Table`/`TableState`, windowed rows, `row_highlight_style` selection, `Connected Peers ({total})` title).
- `src/tui_render_state.rs` — nickname fields, `broadcast_sent_to_peer`, `peer_table_offset`, `get_tab_content` `Info:` prefix.
- `src/tui_helpers.rs` — `peer_table_visible_range` (pure scroll-window math), `peer_table_rows_range`/`peer_table_row` (windowed O(page) row building), `peer_table_column_widths`/`peer_table_header_labels`, `sort_peers_table`/`sort_peers_by_column` (O(n) rebuild).
- `src/nickname.rs` — `get_peer_display_name` + `DISPLAY_NAME_CACHE`, `invalidate_display_name`/`clear_display_names` (`pub(crate)`), `impl_set_peer_field!` macro invalidation. Display name brackets use `fmt::peer_id_suffix` (last 3 chars).
- `src/fmt.rs` — canonical `parse_last_seen_ms`; also `peer_id_suffix`, `format_latency`, `now_timestamp`, `gen_msg_id`.
- `src/peers.rs` — `save_peer` invalidates the cached display name; `record_broadcast_recipients(msg_id, peer_ids)`; `get_network_size`/`get_average_peer_count` (prod-used).
- `src/types.rs` — shared `MAX_MESSAGE_HISTORY`/`MAX_DM_HISTORY` constants (used by the TUI binary).
- `src/db.rs` — `set_db_url`/`reset_db_url` (all four cfg variants) call `nickname::clear_display_names()`.
- `src/bin/tui/state.rs` — `terminal_width`, `peer_table_offset`, `peer_names_warmed`, `kitty_keyboard_active`, `chat_unread_count`, `broadcast_sent_to_peer`; re-exports `MAX_MESSAGE_HISTORY`/`MAX_DM_HISTORY` from `p2p_app::types`.
- `src/bin/tui/main_loop.rs` — startup `supports_keyboard_enhancement()` probe, pushes only `DISAMBIGUATE_ESCAPE_CODES`, and pops it on exit.
- Tests: `tests/unit/unit_tui_tabs.rs`, `tests/unit/unit_tui_helpers.rs`, `tests/unit/unit_bin_tui_click_handlers.rs`, `tests/unit/unit_bin_tui_input_processor.rs`, `tests/unit/unit_bin_tui_render_loop_mod.rs`, `tests/unit/unit_bin_tui_render_loop_layout.rs`, `tests/unit/unit_bin_tui_scroll_handlers.rs`, `tests/tui_render_integration.rs`.
- `Cargo.toml` — `unicode-width` (0.2) dep for display-width measurement of table cells.