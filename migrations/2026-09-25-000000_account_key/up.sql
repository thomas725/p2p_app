-- Device-binding provisioning (§11 / design doc 2026-09-21): the account key
-- that signs device-binding claims for this install's user.
--
-- The per-install `identities` row keeps its libp2p keypair; `account_key`
-- (NULL until this install provisions a user) holds the Ed25519 *account*
-- keypair that minted the self claim and can mint sibling-device claims. Only
-- the account-key holder can sign claims, which is what enforces the
-- hostile-device invariant at the provisioning boundary.
--
-- `down.sql` drops the column (SQLite ≥ 3.35 supports ALTER TABLE DROP COLUMN).

ALTER TABLE identities ADD COLUMN account_key BLOB;