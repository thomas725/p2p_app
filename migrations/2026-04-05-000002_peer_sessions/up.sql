CREATE TABLE IF NOT EXISTS peer_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    concurrent_peers INTEGER NOT NULL DEFAULT 0,
    recorded_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_peer_sessions_recorded_at ON peer_sessions(recorded_at);
