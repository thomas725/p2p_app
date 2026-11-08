-- Persisted network configuration (US-050 follow-up 1): the settings surface
-- for the `P2P_BOOTSTRAP` / `P2P_RELAY` env vars. Each row is one multiaddr of
-- one kind:
--
--   kind = 'bootstrap'  dialed at node start so the node can join the DHT
--   kind = 'relay'      circuit-relay v2 reservation target; needs /p2p/<id>
--
-- `multiaddr` stores the canonical text form; callers validate it by parsing
-- it back into a libp2p Multiaddr, so nothing unparseable reaches the swarm.
-- The unique index makes "add the same peer twice" a no-op instead of a
-- duplicate dial.
CREATE TABLE network_peers (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL,
    multiaddr TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX network_peers_kind_multiaddr
    ON network_peers (kind, multiaddr);
