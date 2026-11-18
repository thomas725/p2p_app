# End-to-end encryption for direct messages (US-042)

**Status: live.** Direct messages between install identities are sealed with a
per-message envelope at the sender and opened by the recipient before anything
is persisted or shown. Broadcasts and group messages are deliberately outside
this lane (see "What is deliberately not encrypted").

## Why this exists at all

libp2p already encrypts every hop with Noise (see `docs/architecture.md`), so a
passive observer between two peers cannot read a DM. What Noise does **not**
cover is anything that stores or relays the body in the clear: a future
offline-delivery/relay path (store-and-forward), or a backup. This lane adds a
second, payload-level envelope sealed for the recipient's device, so the
plaintext never leaves the sender's process.

**At-rest protection is deliberately *not* a goal of this lane.** Protecting
message bodies, peer data and group state on disk should be done at the
**database level** (encrypted SQLite / SQLCipher-style), not per message —
per-message envelopes only protect individual fields and leave metadata,
indexes and the rest of the schema readable. See the backlog item
"Database-level at-rest encryption".

So the remaining justification for this per-recipient envelope is
**store-and-forward / offline caching**: a third node can hold and forward a
sealed blob it must not be able to read. That use case is tracked separately
(see backlog "Store-and-forward / offline caching for direct messages").

## How it works today

- **Key**: the install's Ed25519 identity key (the same keypair that produces
  the peer ID), converted to X25519. The account key is *not* used — sibling
  devices bound into one account do not hold its private half.
- **Recipient key**: recovered directly from the Ed25519 peer ID (an identity
  multihash whose digest is the protobuf public key), so no handshake or
  key-directory lookup is needed. Non-Ed25519 peer IDs fall back to plaintext.
- **Envelope**: fresh ephemeral X25519 keypair per message, ECDH against the
  recipient static key, HKDF-SHA256 (salt = `ephemeral_pub ‖ recipient_pub`,
  info = `HKDF_INFO`) to a ChaCha20-Poly1305 key, random 12-byte nonce, and
  AAD = `HKDF_INFO ‖ 0x00 ‖ msg_id`. Binding `msg_id` into the AAD prevents an
  envelope being replayed under a different wire id.
- **Sealed body** (`SealedDmBody`): content, sender nickname, attachment
  descriptor and the US-006 reply snapshot. Only routing fields (`msg_id`,
  `sent_at`, `group_id`) stay readable on the wire.
- **Send**: `swarm_handler::send_dm` calls `e2e::seal_dm_payload`. On success the
  plaintext `content`/`nickname`/`attachment`/`reply_to` fields are cleared and
  `DirectMessage.encrypted` carries the hex-encoded envelope.
- **Receive**: `handle_request_response` opens `request.encrypted` with
  `e2e::open_incoming` *before* any control check, then continues with the
  decrypted fields and sets `MessageEvent.encrypted`. Malformed/foreign
  envelopes are acked and dropped — they never reach a frontend.
- **Persistence**: `messages.encrypted` stores the real incoming flag; the TUI
  prefixes `[e2e] ` and Flutter shows a lock icon in `MessageBubble` when it is
  set (including after a history reload). Outgoing rows stay `encrypted = 0`
  locally: this device is the sender and holds the plaintext.

## What is deliberately not encrypted

- Broadcasts and group-topic messages (multi-recipient; gossipsub has no
  per-recipient key and the topic itself is public).
- Control traffic on the DM lane: delivery acks, the nickname-only exchange
  sent on connect, and DM-lane group messages — no user content, and keeping
  them readable preserves interoperability with peers that predate this lane.
- Local storage: the DB keeps the plaintext (this device is the intended
  reader) plus the `encrypted` flag.

## Limits

This scheme has **no forward secrecy and no post-compromise security**: a
compromised install identity can decrypt every message ever sealed to it, and
captured envelopes stay decryptable with that key. Key rotation would require
a new envelope version and a way to announce the current key. This is acceptable
for the current store-and-forward motivation; revisit before relying on the
lane for stronger guarantees.

At-rest protection belongs at the database level (see backlog
"Database-level at-rest encryption"), not here.
