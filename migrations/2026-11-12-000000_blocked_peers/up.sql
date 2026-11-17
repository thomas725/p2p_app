-- Peer blocklist / mute (US-054): peers the operator muted. The swarm
-- boundary drops their traffic and refuses to dial them, so a blocked peer
-- cannot persist messages or keep a connection alive — the row is not merely a
-- UI filter.
--
-- `peer_id` is the libp2p peer id in its canonical base58 text form, and is the
-- primary key: blocking is idempotent and unblocking is a single delete.
CREATE TABLE blocked_peers (
    peer_id TEXT NOT NULL PRIMARY KEY,
    blocked_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
