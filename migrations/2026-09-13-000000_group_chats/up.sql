-- Public/private group chats, transported over per-group gossipsub topics
-- (`group:<group_id>`). Public v1 groups are identified by their canonical
-- name (the `group_id` is a stable hash of it, so any node that joins the same
-- name subscribes to the same topic); `is_private` reserves the private-group
-- switch (random group ids + invite/membership) for a later migration.
CREATE TABLE groups (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    group_id TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    is_private INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Peers seen participating in a group (public v1: discovered from traffic;
-- private groups will gate this on membership/invite). Deduplicated per group.
CREATE TABLE group_members (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    group_id TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX group_members_group_peer ON group_members (group_id, peer_id);

-- Group message history. `peer_id` is NULL for messages we sent. Incoming
-- re-deliveries (gossipsub mesh retransmits) are deduplicated by the unique
-- (group_id, msg_id) index.
CREATE TABLE group_messages (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    group_id TEXT NOT NULL,
    content TEXT NOT NULL,
    peer_id TEXT,
    sent INTEGER NOT NULL DEFAULT 0,
    msg_id TEXT,
    sent_at DOUBLE,
    sender_nickname TEXT
);
CREATE UNIQUE INDEX group_messages_group_msg ON group_messages (group_id, msg_id);
CREATE INDEX group_messages_created_at ON group_messages (created_at DESC);