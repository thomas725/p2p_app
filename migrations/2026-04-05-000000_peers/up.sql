-- Peers table: idempotent creation with all columns
-- Uses CREATE TABLE IF NOT EXISTS so safe for fresh DBs
-- For existing databases, missing columns are added by fix_missing_columns() in db.rs
CREATE TABLE IF NOT EXISTS peers (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    peer_id TEXT NOT NULL UNIQUE,
    addresses TEXT NOT NULL,
    first_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    peer_local_nickname TEXT,
    received_nickname TEXT
);

CREATE INDEX IF NOT EXISTS idx_peers_peer_id ON peers(peer_id);
CREATE INDEX IF NOT EXISTS idx_peers_last_seen ON peers(last_seen DESC);