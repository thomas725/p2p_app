-- Messages table may already exist from manual setup
-- This migration handles both fresh databases and existing ones
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    content TEXT NOT NULL,
    peer_id TEXT,
    topic TEXT NOT NULL DEFAULT 'test-net',
    sent INTEGER NOT NULL DEFAULT 0,
    is_direct INTEGER NOT NULL DEFAULT 0,
    target_peer TEXT
);

CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_topic ON messages(topic);