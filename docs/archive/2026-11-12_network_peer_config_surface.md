# Persisted bootstrap/relay config surface (2026-11-12)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

US-050 follow-up 1 closed: the node's two internet-facing entry points —
Kademlia bootstrap peers and circuit-relay targets — stopped being env-var
only. Both are now rows in a `network_peers` table, visible and editable from
every frontend, and read by all three entry points (TUI, mobile/Flutter,
`p2p_chat_server`) without an env var. Shipped in `b11d03a` + `61f9f2a`.

## The shape

- Migration `migrations/2026-11-08-000000_network_peers/`: table with
  `id`, `kind` (`bootstrap` | `relay`), `multiaddr`, `created_at`, plus a unique
  index on `(kind, multiaddr)` so re-adding the same address is a no-op
  (`on_conflict_do_nothing`) instead of an error — the UI stays a plain "add".
- `src/network_config.rs` is the single owner: `NetworkPeerKind`
  (`as_str`/`parse`), `NetworkPeer` (camelCase JSON, the exact shape FRB emits
  and `src/web/methods.rs` forwards), `validate_network_peer`,
  `list_network_peers`, `add_network_peer`, `remove_network_peer`, and the
  node-start readers `stored_bootstrap_addrs` / `stored_relay_addrs`.
- Diesel artifacts are hand-maintained in this repo: `src/generated/schema.rs`
  gained the `network_peers` block and the
  `allow_tables_to_appear_in_same_query!` entry, with `NetworkPeerRow` /
  `NewNetworkPeer` added by hand (`diesel_ext` is not on crates.io, so
  `diesel_generate.sh` must not be run).

## Validation

Everything is validated by parsing back into a `Multiaddr`, and the *canonical*
text form is what gets stored — so `/ip4/1.2.3.4/tcp/4001` and its long forms
collapse. Relay entries must additionally carry `/p2p/<peer-id>` (a reservation
is requested from one specific relay). That check uses a local `peer_id_in`
helper rather than `relay::relay_peer_id`, so the rule holds even in builds
without the `relay` feature. Errors are human-readable and rendered verbatim by
both frontends (TUI popup, Flutter dialog).

## Precedence (asymmetric on purpose)

| List | Order |
|---|---|
| bootstrap | `P2P_BOOTSTRAP` (set ⇒ wins, even empty = disabled) → persisted `bootstrap` rows → public IPFS nodes, only with `P2P_PUBLIC_DHT=1` |
| relay | `P2P_RELAY` **∪** persisted `relay` rows, deduplicated |

The env var wins for bootstrap so an operator can override or disable what the
UI holds; relays are unioned because a relay is an explicit opt-in, and adding
one in Settings must never drop an env-configured one.

Both are read **only at node start**. The Settings UI and the saved-add toast
state "applies on next node start"; there is no live re-configuration path.
`stored_*_addrs()` is best-effort — a database error returns an empty list so
the node still starts with the env / public-DHT fallbacks.

## Frontend surface

- **TUI**: Settings tab renders a `Network peers (N)` block with a `>` highlight.
  `b` / `r` open the shared add prompt (`InputMode::AddingNetworkPeer` +
  `AppState::network_peer_kind`, so the input title says which kind), `x`
  removes the highlighted row, Up/Down/Home/End move the highlight. The prompt
  is checked *before* the ordinary Settings shortcuts, mirroring the Contacts
  search rule, so typing `b` inside the box edits the address instead of
  reopening the prompt.
- **Flutter**: a "Network peers" card on Settings with the same list, an add
  dialog (kind dropdown + address field; validation stays in Rust), and a
  per-row delete.
- **Web**: the card works unchanged over the JSON transport because
  `listNetworkPeers` / `addNetworkPeer` / `removeNetworkPeer` mirror the FRB
  surface 1:1 in `src/web/methods.rs`, with the usual no-param drift-list and
  dispatch-routing test entries.

## What this unblocked

The manual dial shipped earlier as US-053 is the *one-off* reachability tool
(Settings `d`, Flutter's "Connect to address" card, `dialPeer`); this slice is
the *persistent* list. Together they are the whole follow-up-1 item from the
2026-11-03 US-050 write-up. Follow-ups 2 (relay reservation triggering) and 3
(AutoNAT-driven addressing) had already shipped on 2026-11-08.
