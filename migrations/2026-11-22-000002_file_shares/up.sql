-- File shares / folder sync (share lane). One row per share this install owns
-- or has accepted from a peer. `kind`: 0 = private (grant-gated), 1 = public;
-- public shares may opt into writes with `public_write`. `local_root` is the
-- real directory bound on this install (NULL until the user binds one),
-- `manifest_json` the last accepted snapshot and `revision` its optimistic
-- concurrency token. A write is only accepted when its base revision still
-- matches the stored one.
CREATE TABLE shares (
    share_id TEXT NOT NULL PRIMARY KEY,
    owner_user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    kind INTEGER NOT NULL DEFAULT 0,
    public_write INTEGER NOT NULL DEFAULT 0,
    local_root TEXT,
    revision INTEGER NOT NULL DEFAULT 0,
    manifest_json TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Per-target-user permissions on a private share (identity lane §11: user IDs
-- are verified via signed device bindings, never claimed by a requester).
-- `mode`: 0 = read-only, 1 = read-write.
CREATE TABLE share_grants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    share_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    mode INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (share_id, user_id)
);

-- Denormalised current snapshot of each share (one row per relative path).
-- Replaced atomically whenever a new manifest is accepted. `file_id` is the
-- content address in `<db>.files`, so this table also tells attachment GC
-- which blobs a live share still needs.
CREATE TABLE share_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    share_id TEXT NOT NULL,
    path TEXT NOT NULL,
    file_id TEXT NOT NULL,
    size BIGINT NOT NULL,
    mime TEXT,
    UNIQUE (share_id, path)
);

CREATE INDEX idx_share_entries_file_id ON share_entries (file_id);
