# Archive

Historical documents: completed feature slices, superseded designs, and one-off
research. Kept for context — **not maintained**, and nothing here describes
current behaviour.

Each file keeps its original dated name, because the date is part of the
record. Every file carries an `ARCHIVED` banner at the top.

## Index

### Completed feature slices

| Doc | What it records | Closed |
|---|---|---|
| [2026-08-29_tui_peerinfo_tab_and_peer_table.md](2026-08-29_tui_peerinfo_tab_and_peer_table.md) | TUI PeerInfo tab + the sortable peer table | 2026-08-29 |
| [2026-09-01_tui_polish_and_peer_perf.md](2026-09-01_tui_polish_and_peer_perf.md) | ConnectedTracker, paging, stateful `Table`, display-name memoization | 2026-09-01 |
| [2026-09-02_mobile_frb_flutter_parity.md](2026-09-02_mobile_frb_flutter_parity.md) | Sortable peer table, single-round-trip stats, FRB 2.13.0 | 2026-09-02 |
| [2026-09-03_broadcast_recipients_and_dioxus_removal.md](2026-09-03_broadcast_recipients_and_dioxus_removal.md) | The `broadcast_recipients` rework; Dioxus removal | 2026-09-03 |
| [2026-10-20_delivery_receipts_parity.md](2026-10-20_delivery_receipts_parity.md) | DB-computed `confirmed` ticks in Flutter + web | 2026-10-20 |
| [2026-10-20_log_event_parity.md](2026-10-20_log_event_parity.md) | `file_received` / `peer_expired` logged in every frontend | 2026-10-20 |
| [2026-10-20_message_latency_parity.md](2026-10-20_message_latency_parity.md) | Per-message round-trip time in Flutter + web | 2026-10-20 |
| [2026-10-20_peerinfo_stats_rows.md](2026-10-20_peerinfo_stats_rows.md) | DM/broadcast counts on the Peer Info page | 2026-10-20 |
| [2026-10-21_contacts_search.md](2026-10-21_contacts_search.md) | The Contacts search affordance in both frontends | 2026-10-21 |
| [2026-10-26_attachment_size_labels.md](2026-10-26_attachment_size_labels.md) | Human-readable `(2 KiB)` attachment chips from one Rust formatter | 2026-10-26 |
| [2026-11-08_connection_quality.md](2026-11-08_connection_quality.md) | Per-peer ping RTT/loss on the Peer Info page (live-only samples) | 2026-11-08 |
| [2026-11-12_network_peer_config_surface.md](2026-11-12_network_peer_config_surface.md) | Persisted bootstrap/relay multiaddrs editable in all three frontends | 2026-11-12 |
| [2026-11-12_peer_blocklist.md](2026-11-12_peer_blocklist.md) | Peer blocklist / mute enforced at the swarm boundary (US-054) | 2026-11-12 |
| [2026-11-17_keyboard_shortcuts_help.md](2026-11-17_keyboard_shortcuts_help.md) | TUI `?` overlay + Flutter Help / controls dialog (US-029) | 2026-11-17 |
| [2026-11-17_local_message_deletion.md](2026-11-17_local_message_deletion.md) | Local-only tombstones for sent messages (US-005 deletion half) | 2026-11-17 |
| [2026-11-18_reply_quote.md](2026-11-18_reply_quote.md) | Immutable reply snapshots carried on the wire and quoted in all three frontends (US-006) | 2026-11-18 |
| [2026-11-18_us-042_e2e_dm.md](2026-11-18_us-042_e2e_dm.md) | Sealed DM bodies end to end, with a persisted flag and lock markers (US-042) | 2026-11-18 |
| [2026-11-19_store_and_forward_dm_outbox.md](2026-11-19_store_and_forward_dm_outbox.md) | Sealed-DM outbox: retry on reconnect, ack cleanup, dedup, expiry | 2026-11-19 |
| [2026-11-20_local_message_edits.md](2026-11-20_local_message_edits.md) | Local-only sent-message edits with a persisted `(edited)` marker (US-005 edit half) | 2026-11-20 |
| [2026-11-21_typing_indicators.md](2026-11-21_typing_indicators.md) | Fire-and-forget DM typing indicators on a dedicated lane (US-009) | 2026-11-21 |
| [2026-11-21_message_reactions.md](2026-11-21_message_reactions.md) | Persistent per-message reactions synced on a dedicated lane (US-008) | 2026-11-21 |
| [2026-11-21_lan_only_mode.md](2026-11-21_lan_only_mode.md) | Persisted LAN-only toggle gating DHT, relays and internet bootstrap at node start (US-051) | 2026-11-21 |
| [2026-11-21_themes.md](2026-11-21_themes.md) | Rust-owned dark/light + accent palette shared by the TUI and Flutter/web (US-025/US-026) | 2026-11-21 |
| [2026-11-21_notifications.md](2026-11-21_notifications.md) | Rust-owned notification payload/policy with thin TUI, Android and browser delivery adapters (US-027/US-028) | 2026-11-21 |
| [2026-11-22_call_media_transport.md](2026-11-22_call_media_transport.md) | Rust-owned μ-law voice frame transport on a dedicated lane with jitter buffer/mute + TUI/Flutter parity (US-044) | 2026-11-22 |
| [2026-11-23_tui_clipboard.md](2026-11-23_tui_clipboard.md) | TUI `Ctrl+C` copies the newest message via a pure OSC 52 sequence builder | 2026-11-23 |
| [2026-11-23_tui_log_copy_all.md](2026-11-23_tui_log_copy_all.md) | TUI Log `Ctrl+C` copies the whole buffer like Flutter's "Copy All" | 2026-11-23 |
| [2026-11-23_tui_dm_group_unread.md](2026-11-23_tui_dm_group_unread.md) | TUI DM/group unread banners and scroll fidelity while reading history | 2026-11-23 |
| [2026-11-02_sibling_device_claims.md](2026-11-02_sibling_device_claims.md) | Sibling devices may sign device-binding claims for the same user (P0 decision) | 2026-11-02 |
| [2026-11-12_public_bootstrap_discovery.md](2026-11-12_public_bootstrap_discovery.md) | Internet-wide discovery: Kademlia DHT, NAT traversal, network details, configurable bootstrap/relay peers (US-050) | 2026-11-12 |

### Completed planning

| Doc | What it records | Closed |
|---|---|---|
| [2026-04-17_user_stories.md](2026-04-17_user_stories.md) | The original user stories and acceptance criteria | 2026-10-21 — the still-open ones moved to [../backlog.md](../backlog.md) |
| [2026-09-02_backlog.md](2026-09-02_backlog.md) | The cross-frontend engineering backlog, all 12 items | 2026-10-21 — the still-open ones moved to [../backlog.md](../backlog.md) |
| [2026-04-19_PLAN.md](2026-04-19_PLAN.md) | Refactoring the TUI into decoupled Tokio tasks | implemented |
| [2026-09-07_archived_agents_context.md](2026-09-07_archived_agents_context.md) | History removed from `AGENTS.md` when it was reduced to rules only | 2026-09-07 |
| [2026-06-08_prompt_log.md](2026-06-08_prompt_log.md) | A raw chronological log of prompts | raw history, never a reference |

### Superseded designs

| Doc | Superseded by | Note |
|---|---|---|
| [2026-05-01_clippy_decision_framework.md](2026-05-01_clippy_decision_framework.md) | `AGENTS.md` — "Clippy allowance policy" | The general framework still holds; its concrete "current project examples" describe a policy state that was retired on 2026-09-28. |
| [2026-10-21_github_workflows_setup.md](2026-10-21_github_workflows_setup.md) | [`../ci_pipeline.md`](../ci_pipeline.md) | Described `ci.yml`, `coverage.yml`, and `documentation.yml` — none of which exist. The workflows it documented were replaced by the five that ship today. |
| [2026-04-17_dual_frontend_architecture.md](2026-04-17_dual_frontend_architecture.md) | [`../architecture.md`](../architecture.md) | Written for the TUI + Dioxus split; Dioxus was removed on 2026-09-03. |
| [2026-04-14_keyboard_modifier_detection.md](2026-04-14_keyboard_modifier_detection.md) | [`../../KEYBOARD_SHORTCUTS.md`](../../KEYBOARD_SHORTCUTS.md) | Its binding table still holds; the enhancement-flag snippet and the mouse-capture field path do not. |
| [2026-04-17_tui_layout_rows.md](2026-04-17_tui_layout_rows.md) | the code itself | The row model it describes (notification row, two chat list-header rows) was superseded. |
| [2026-04-17_dioxus_implementation.md](2026-04-17_dioxus_implementation.md) | — | Dioxus was removed on 2026-09-03. |
| [2026-04-17_dioxus_research.md](2026-04-17_dioxus_research.md) | — | Dioxus was removed on 2026-09-03. |
| [2026-04-04_feature_sizes.md](2026-04-04_feature_sizes.md) | — | Every size number predates the default feature set gaining `web` → `mobile` (FRB + axum). |
| [2026-08-17_android_development_plan.md](2026-08-17_android_development_plan.md) | [`../identity.md`](../identity.md), the Android app itself | Written when no Android code existed. |
| [2026-08-17_apk_sharing_strategy.md](2026-08-17_apk_sharing_strategy.md) | `MainActivity.shareApk()` | The share intent shipped. |
| [2026-08-17_flutter_vs_native_option_h.md](2026-08-17_flutter_vs_native_option_h.md) | [`../identity.md`](../identity.md) | The Flutter + `flutter_rust_bridge` decision it weighs was made. |

## Why archive instead of delete

These documents encode *why* the code looks the way it does. Deleting them
loses the reasoning that would otherwise have to be rediscovered — and several
of them record decisions that were expensive to reach (the identity root of
trust, the Dioxus removal, the `broadcast_recipients` rework).
