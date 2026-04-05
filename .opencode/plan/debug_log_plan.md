# Debug Log Improvements Plan

## Tracing Denylist

| Module | Example Output | Why Filtered |
|--------|---------------|-------------|
| `multistream_select` | `Dialer: Proposed protocol: /noise`, `Negotiated: Received confirmation for protocol: /yamux/1.0.0` | Protocol negotiation internals. Success/failure is visible from higher-level swarm events (`ConnectionEstablished`, `IncomingConnectionError`). |
| `yamux::connection` | `new connection: 75cb6380 (Client)`, `sending ping 2627618332`, `received pong, estimated round-trip-time 5.7ms` | Stream multiplexing internals. RTT pings are noise for app-level debugging. Connection state is visible via libp2p_swarm events. |
| `libp2p_core::transport::choice` | Massive Rust type names spanning hundreds of characters on dial failure | Completely unreadable due to nested generic type names. Redundant with higher-level dial errors from `libp2p_swarm` and `libp2p_tcp`. |
| `libp2p_mdns::behaviour::iface` | `creating instance on iface address address=10.201.0.2` | Only fires once at startup. We already see listen addresses from `libp2p_tcp` and `libp2p_quic::transport`. |

## Modules We Keep

| Module | Why Kept |
|--------|----------|
| `libp2p_mdns::behaviour` (not iface) | Peer discovery events - essential for understanding when peers are found |
| `libp2p_swarm` | Connection lifecycle, listener addresses, dial results - core networking state |
| `libp2p_gossipsub::behaviour` | Mesh changes, heartbeats, peer subscriptions - message routing state |
| `libp2p_tcp` | Dial attempts, listen addresses - transport layer visibility |
| `libp2p_quic::transport` | Listen addresses - transport layer visibility |

## Duplicate Manual Logging to Remove

These are covered by libp2p tracing events and create duplicate lines:

| Manual Log | Tracing Replacement |
|-----------|-------------------|
| `Connection established: {} (conn: {:?})` | `libp2p_swarm: Connection established peer={} endpoint={}` |
| `Disconnected from: {} (conn: {:?}, cause: {:?})` | `libp2p_swarm: Connection closed peer={} cause={}` |
| `mDNS discovered: {} at {}` | `libp2p_mdns::behaviour: discovered peer on address peer={} address={}` |
| `Incoming connection {:?} from {:?}` | `libp2p_swarm: Incoming connection from remote at local` |
| `Dialing: {}` | `libp2p_tcp: dialing address address={}` |

## Debug Tab Scrolling

- Add `debug_scroll_offset: usize` state variable
- Handle `Up`/`Down` keys when active_tab == 3 (Debug)
- Render visible portion: `logs[scroll_offset..scroll_offset+visible_lines]`
- Show scroll position in title: `Debug [15/234]`

## ANSI Escape Code Stripping

The `tracing_subscriber::fmt::layer()` outputs ANSI color codes by default. These leak into the TUI buffer as raw characters like `[0m`, `6:34m`, etc. The `TracingWriter` must strip these before writing to the log buffer.

---

## Key Learnings from Development

### Multi-instance Identity Collision
All instances sharing the same `sqlite.db` load the same libp2p identity keypair. Two peers with identical identities cannot connect.
**Fix:** Generate ephemeral identity when `DATABASE_URL` is not set.

### Gossipsub Mesh Requirements
- `mesh_n` must be <= number of available peers, otherwise mesh stays empty
- For 2-peer networks: `mesh_n=1, mesh_n_low=1, mesh_n_high=2`
- `flood_publish=true` for Small networks eliminates IWANT/IHAVE delay
- Heartbeat interval of 500ms for fast propagation in small networks

### mDNS Discovery Asymmetry
mDNS is inherently asymmetric - Peer B discovers Peer A immediately if A is already responding to queries. But Peer A won't discover Peer B until the next query cycle. The rust-libp2p mDNS implementation has a probing phase (`INITIAL_TIMEOUT_INTERVAL: 500ms`) before queries start.
**Fix:** Dial known peers from database on startup.

### Stale Addresses in Database
QUIC ports change on every restart. Dialing old QUIC addresses causes `BrokenPipe` and `APPLICATION_ERROR` errors.
**Fix:** Only dial TCP addresses from database (more stable across restarts).

### Simultaneous Dial Errors
When both peers discover each other via mDNS simultaneously, both attempt to dial, causing one connection to succeed and the other to fail with `ConnectionClosed(APPLICATION_ERROR)`. This is expected libp2p behavior - the duplicate connection is rejected.
**Fix:** Mark these as "expected" in logs to reduce noise.

### Peer List Deduplication
mDNS reports the same peer on multiple addresses (TCP + QUIC) in a single event batch. Dialing all addresses causes redundant connection attempts.
**Fix:** The swarm handles this internally, but we should avoid excessive logging.

### Network Size Optimization
Gossipsub config should adapt based on historical peer count:
- **Small (0-3):** 500ms heartbeat, mesh_n=1, flood_publish=true
- **Medium (4-15):** 1s heartbeat, mesh_n=6
- **Large (16+):** 2s heartbeat, mesh_n=8
