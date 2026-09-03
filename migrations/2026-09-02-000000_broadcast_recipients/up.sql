-- Records, per broadcast *we* sent, which peers we transmitted it to, and once
-- they acknowledge, when they confirmed receipt. Drives the "Broadcast"
-- (broadcasts sent-to-peer) counter in the peers table, replacing the dormant
-- `peers.broadcasts_sent` aggregate counter.
CREATE TABLE broadcast_recipients (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    msg_id TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    sent_at DOUBLE NOT NULL,
    confirmed_at DOUBLE
);
CREATE UNIQUE INDEX broadcast_recipients_msg_peer
    ON broadcast_recipients (msg_id, peer_id);