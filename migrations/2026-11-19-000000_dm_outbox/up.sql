-- Store-and-forward DM outbox (backlog item): one row per sealed DM that is
-- still waiting for delivery. Only the *sealed* wire message is stored — the
-- envelope's plaintext fields are deliberately empty, so a database dump of
-- this table reveals routing fields and ciphertext, never the body.
--
-- The row is created when `send_dm` seals an outgoing DM, deleted when the
-- recipient's ack (`ack_for = msg_id`) comes back, and retried on reconnect /
-- a periodic sweep. `(recipient, msg_id)` is unique so a resend or a replay
-- cannot create a second row; `expires_at` bounds retention.
CREATE TABLE dm_outbox (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    recipient_peer_id TEXT NOT NULL,
    msg_id TEXT NOT NULL,
    -- Full sealed DirectMessage as JSON (content/nickname/attachment/reply_to
    -- empty, `encrypted` populated). No plaintext body is ever written here.
    envelope_json TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX dm_outbox_recipient_msg_id
    ON dm_outbox (recipient_peer_id, msg_id);
CREATE INDEX dm_outbox_recipient ON dm_outbox (recipient_peer_id);
CREATE INDEX dm_outbox_expires_at ON dm_outbox (expires_at);
