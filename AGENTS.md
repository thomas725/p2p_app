## Goal
- Fix broken TUI click handlers (peer-click bug + missing message-click) and port the Flutter `PeerInfoScreen` into the TUI as a new PeerInfo tab.
- Adapt the TUI peer list to mirror Flutter's sortable peer table (columns Name, DM count, Broadcast count, Last seen; each sortable by key or header click).

## Constraints & Preferences
- Toolchain: **nightly** (`rustup override set nightly` in `/home/user/project`).
- All code must be clippy-clean with the project's strict pedantic config: `cargo ct` (alias for `cargo clippy --all-targets --all-features`) must exit 0 with no warnings.
- User decisions: peer click in Peers tab **keeps opening the DM tab**; message click in Chat/Log/DM **opens the sender's/partner's PeerInfo**.

## Progress
### Done
- Strict clippy-lints project: committed earlier (`38241ed`). Settings tab committed earlier (`33ab735`).
- **Fixed peer-row click bug**: `chat_area_height` (and new `terminal_width`) are now set every frame in the render loop (`src/bin/tui/render_loop/mod.rs:160`), so `handle_peer_row_click`'s guard actually fires. Also fixed an off-by-one so the last peer row is clickable (`mouse_row <= max_row`).
- **Added message click → PeerInfo**: `handle_message_click` (`src/bin/tui/click_handlers.rs`) maps the clicked row to the message via `calc_visible_strings` + `row_to_visible_index` + `count_lines`, then opens the sender's PeerInfo tab. Direct-tab clicks open the partner's PeerInfo.
- **Added `TabContent::PeerInfo(String)`** (`src/tui_tabs.rs`): `peer_info_tabs` Vec on `DynamicTabs`; `add_peer_info_tab` / `remove_peer_info_tab` (dedup, index computed after Chat/Peers/DMs); wired into `all_titles`, `tab_index_to_content`, `total_tab_count`, and `peer_id()`.
- **Rendered PeerInfo tab** (`src/tui_render.rs::render_peer_info_content`): ported Flutter fields — display name, peer ID, nickname origin (local/received/generated petname), local + received nickname, first-seen/last-seen, "Press Enter to open direct message". `TuiRenderState` gained `local_nicknames`/`received_nicknames`/`self_nicknames_for_peers` (populated in `app_state_to_render_state`); `get_tab_content` recognizes the `Info: ` title prefix.
- **Input wiring** (`src/bin/tui/input_processor.rs`): Enter on PeerInfo → opens DM (via extracted `handle_enter_key` branch); `i` key opens PeerInfo for the selected peer (Peers) or DM partner (Direct) via new `open_peer_info_for_active_tab`; Esc is a no-op on PeerInfo (extracted `handle_esc_key`). Scroll handlers (`scroll_handlers.rs`) treat PeerInfo as a no-op.
- **PeerInfo tabs are closeable**: the `(X)` close button in `handle_tab_click` (`src/bin/tui/click_handlers.rs`) now also matches `TabContent::PeerInfo` (via `remove_peer_info_tab`), and Ctrl+W (`handle_close_dm_tab` in `src/bin/tui/input_processor.rs`) closes both DM and PeerInfo tabs.
- **Tests added** (all passing): `tui_tabs` peer-info add/dedup/remove + `peer_id`; click-handler message-click opens PeerInfo and own-message no-op (fixed a pre-existing failing assertion `tab_titles.len() == 4` in `unit_bin_tui_render_loop_mod.rs`); `i`-key and Enter-on-PeerInfo in `input_processor`.
- **Verification**: `cargo ct` → exit 0, 0 errors, 0 warnings. `cargo test --bin p2p_chat_tui --all-features` → 193 passed. `cargo test --lib --all-features` → 229 passed.

### In Progress
- (none)

### Blocked
- (none)

## Key Decisions
- PeerInfo is a **transient dynamic tab** (like DM), not a fixed tab: inserted after DMs, before Log/Settings. Dedup by peer ID.
- Click row→message mapping reuses library helpers; `calc_visible_strings` returns `(visible, start)` where `start` is the first visible message index (for auto-scroll it equals `total - visible`), used to map the relative row back to the absolute message index.
- `terminal_width` stored on `AppState` (set in render loop via `usize::from(f.area().width)`) so click mapping matches wrapping; default 0 treated as width 1 (only before first render).

## Next Steps
- (none outstanding — feature complete and verified)

## Critical Context
- `chat_area_height` = `f.area().height - 11` (1 tab + 1 peer-info + 5 input + 1 shortcut + 1 status = 9, minus 2 border). `terminal_width` = `f.area().width`.
- `render_frame` match (`src/bin/tui/render_loop/mod.rs:120`) now has a `TabContent::PeerInfo(peer_id)` arm calling `render_peer_info_content`.
- `handle_mouse_left_click(state, mouse_row, mouse_column, is_peers_tab)` keeps its 4-arg signature (tests depend on it); message clicks are gated on the active tab being Chat/Log/Direct.
- `calc_visible_strings(&VecDeque<String>, auto_scroll, scroll_offset, text_width, usable_height) -> (usize, usize)` and `row_to_visible_index(&[usize], first_content_row, click_row) -> Option<usize>` are the helpers used for hit-testing.

## Relevant Files
- `src/tui_tabs.rs` — `TabContent::PeerInfo`, `peer_info_tabs`, add/remove/titles/content/count.
- `src/bin/tui/click_handlers.rs` — `handle_message_click`, fixed peer-click guard.
- `src/bin/tui/input_processor.rs` — `open_peer_info_for_active_tab`, `handle_esc_key`, PeerInfo Enter branch, `i` key.
- `src/bin/tui/scroll_handlers.rs` — PeerInfo no-op arms.
- `src/bin/tui/render_loop/mod.rs` — sets `chat_area_height`/`terminal_width`; renders PeerInfo; `app_state_to_render_state` nickname maps.
- `src/tui_render.rs` — `render_peer_info_content`.
- `src/tui_render_state.rs` — nickname fields, `get_tab_content` `Info:` prefix.
- `src/bin/tui/state.rs` — `terminal_width` field.
- Tests: `tests/unit/unit_tui_tabs.rs`, `tests/unit/unit_bin_tui_click_handlers.rs`, `tests/unit/unit_bin_tui_input_processor.rs`, `tests/unit/unit_bin_tui_render_loop_mod.rs`.
- Reference: Flutter `PeerInfoScreen` at `apps/flutter_app/lib/main.dart:1141-1494`.
