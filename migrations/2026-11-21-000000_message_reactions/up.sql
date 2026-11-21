-- Message reactions (US-008). A reaction is keyed by the wire `msg_id`, so the
-- same table serves broadcast, direct and group messages without a discriminator.
-- `peer_id` is the reactor's transport peer id (an account may own several
-- devices, so reactions stay per device, mirroring the blocklist). The unique
-- constraint makes a re-applied reaction idempotent.
CREATE TABLE IF NOT EXISTS message_reactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    msg_id TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    emoji TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (msg_id, peer_id, emoji)
);

-- Every query loads all reactions for a message id, so one index covers both
-- the batched history hydration and the live event lookup.
CREATE INDEX IF NOT EXISTS message_reactions_msg_id ON message_reactions (msg_id);
