-- Identity lane (§11 of the backlog; design doc 2026-09-21). Three tables
-- that lift the app layer above the per-install keypair:
--
--   users           stable account (root of trust). `account_pubkey` is the
--                   BLOB account key that signs every device-binding claim.
--   device_bindings signed device→user claims. A row only exists after the
--                   claim's signature verifies against *that user's* account
--                   pubkey (the hostile-device invariant: a device can never
--                   mint a binding into a foreign user's device-set, because
--                   it lacks the foreign account key).
--   contacts        user→user adjacency (owner's contact list; device-agnostic,
--                   unlike `peers` which is install/device-scoped).
--
-- `down.sql` drops in reverse dependency order (contacts → device_bindings →
-- users).

CREATE TABLE IF NOT EXISTS users (
    user_id TEXT NOT NULL PRIMARY KEY,
    account_pubkey BLOB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    self_username TEXT,
    self_nickname TEXT
);

CREATE INDEX IF NOT EXISTS idx_users_self_username ON users (self_username);

CREATE TABLE IF NOT EXISTS device_bindings (
    claim_id TEXT NOT NULL PRIMARY KEY,
    user_id TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    device_pubkey BLOB NOT NULL,
    claim BLOB NOT NULL,
    bound_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users (user_id) ON DELETE CASCADE,
    FOREIGN KEY (peer_id) REFERENCES peers (peer_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_device_bindings_user ON device_bindings (user_id);
CREATE INDEX IF NOT EXISTS idx_device_bindings_peer ON device_bindings (peer_id);

CREATE TABLE IF NOT EXISTS contacts (
    owner_user_id TEXT NOT NULL,
    contact_user_id TEXT NOT NULL,
    nickname_override TEXT,
    added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen TIMESTAMP,
    PRIMARY KEY (owner_user_id, contact_user_id),
    FOREIGN KEY (owner_user_id) REFERENCES users (user_id) ON DELETE CASCADE,
    FOREIGN KEY (contact_user_id) REFERENCES users (user_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_contacts_owner ON contacts (owner_user_id);
CREATE INDEX IF NOT EXISTS idx_contacts_contact ON contacts (contact_user_id);
