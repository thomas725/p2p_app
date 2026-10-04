-- Content-addressed attachment store (files lane §12). The bytes live on disk
-- under a per-DB `<db>.files` directory keyed by file_id = SHA-256 hex of the
-- content; this table records what we hold plus the delivery-facing metadata
-- (name, size, mime, direction, absolute path) so serving and the frontends'
-- open/save affordances have exactly one lookup. direction: 0 = received,
-- 1 = sent.
CREATE TABLE files (
    file_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    size BIGINT NOT NULL,
    mime TEXT,
    direction INTEGER NOT NULL DEFAULT 0,
    path TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Messages can carry one optional attachment descriptor (files lane §12),
-- serialized as JSON into these columns; NULL = no attachment. Backward
-- compatible: old rows keep NULL and old binaries simply ignore the column.
ALTER TABLE messages ADD COLUMN attachment_json TEXT;
ALTER TABLE group_messages ADD COLUMN attachment_json TEXT;