# Public bootstrap nodes / internet discovery (US-050, 2026-11-02 → 11-12)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/architecture.md](../architecture.md),
> [docs/README.md](../README.md).

This closes **US-050**. Discovery started mDNS-only (LAN-scoped by
construction); the finished feature reaches the internet via a Kademlia DHT with
NAT traversal and a configurable set of bootstrap/relay entry points.

## What shipped

- **Stage 1 — Kademlia + Identify** (`ecc75e7`): new `kad` feature (default +
  mobile) enabling `libp2p/kad` + `libp2p/identify`; `AppBehaviour` gains `kad`
  (MemoryStore) and `identify`; Identify-received listen addresses feed the DHT
  and are persisted for redial; `kad.bootstrap()` runs on connect; startup dials
  `P2P_BOOTSTRAP` multiaddrs.
- **Stages 2-4 — NAT traversal** (`9724a4b`): `relay` feature (circuit-relay v2
  server + client, swarm wrapped with `with_relay_client`); `nat` feature
  (AutoNAT + DCUtR); all four behaviours cfg-gated in `AppBehaviour` with
  catch-all handling.
- **Stage 5 — network details view** (`4439bad`, `a86a109`): `src/net_diag.rs`
  records listen/external addresses, AutoNAT status/probes, DCUtR punches, relay
  reservations/circuits, Kademlia mode/routing/bootstrap and DHT announcements;
  OS interfaces via `network-interface` (5 s cache). A rate-limited (60 s)
  `/p2p-chat/peer/<peer_id>` DHT record makes presence real. TUI Settings block,
  Flutter Settings card and the `getNetworkDetails` web method all surface it.
- **Public IPFS DHT** (`0a2603a`, `408db4f`): default bootstrap falls back to
  public IPFS nodes when `P2P_BOOTSTRAP` is unset; Kademlia forced to
  `Mode::Client`; Identify protocol renamed to `/ipfs/id/1.0.0` for
  interoperability; the relay server only accepts reservations when
  `P2P_RELAY_SERVER=1` so we are never an open relay. `P2P_PUBLIC_DHT=1` opts in.
- **Follow-up 1 — config surface** (`b11d03a`, `61f9f2a`, `916aeb9`): bootstrap
  and relay peers persist in the `network_peers` table (`kind` =
  `bootstrap` | `relay`, unique on `(kind, multiaddr)`) and are editable in the
  TUI (`b` / `r` / `x`), Flutter ("Network peers" card) and web (mirrored JSON
  methods). Entries are validated as `Multiaddr`; relays require
  `/p2p/<peer-id>`. Lists are read at node start only. Bootstrap precedence:
  `P2P_BOOTSTRAP` (set ⇒ wins, even empty) → persisted rows → public IPFS nodes
  with `P2P_PUBLIC_DHT=1`; relays are the union of env and persisted rows.
- **Follow-up 2 — relay reservation triggering** (`428c92b`): `src/relay.rs`
  parses `P2P_RELAY` and builds the `/p2p-circuit` listen address; the swarm
  handler listens on every configured relay at node start and when one connects;
  `ListenerClosed`/`ListenerError` free the relay for a later retry. Renewal is
  the relay client's own while the listener stays open.
- **Follow-up 3 — AutoNAT-driven addressing** (`428c92b`): when AutoNAT says
  `Private` and a reservation is held, only completed circuit addresses are
  advertised; otherwise external addresses come first with circuits appended.
  Identify advertises the circuit listener automatically.

The manual one-off dial is the separate US-053 item. All stages and follow-ups
shipped; nothing here is blocked on an external decision.
