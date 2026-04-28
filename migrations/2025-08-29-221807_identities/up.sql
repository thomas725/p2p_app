-- Your SQL goes here
CREATE TABLE identities (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    key BLOB NOT NULL,
    last_tcp_port INTEGER,
    last_quic_port INTEGER,
    self_nickname TEXT
);
