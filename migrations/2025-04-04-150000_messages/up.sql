-- Your SQL goes here
CREATE TABLE messages (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    content TEXT NOT NULL,
    is_outgoing INTEGER NOT NULL DEFAULT 1,
    peer_id TEXT,
    topic TEXT NOT NULL DEFAULT 'test-net'
);

CREATE INDEX idx_messages_created_at ON messages(created_at DESC);
CREATE INDEX idx_messages_topic ON messages(topic);