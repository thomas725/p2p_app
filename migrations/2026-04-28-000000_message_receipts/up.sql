-- Track per-peer receipt confirmations for outgoing messages.
-- Uses epoch seconds for confirmed_at for millisecond precision.

CREATE TABLE IF NOT EXISTS message_receipts (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    msg_id TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    kind INTEGER NOT NULL, -- 0=broadcast, 1=dm
    confirmed_at REAL NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (msg_id, peer_id, kind)
);

CREATE INDEX IF NOT EXISTS idx_message_receipts_msg_id ON message_receipts(msg_id);
CREATE INDEX IF NOT EXISTS idx_message_receipts_peer_id ON message_receipts(peer_id);
CREATE INDEX IF NOT EXISTS idx_message_receipts_kind ON message_receipts(kind);

