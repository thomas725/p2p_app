# End-to-end encryption for direct messages (US-042)

**Status: scaffolding only.** The crypto module, the wire field and the DB column
exist and compile, but the send/receive paths do **not** yet seal or open
messages. No DM is encrypted end to end today. See "What is still missing" below.

## Why this exists at all

libp2p already encrypts every hop with Noise (see `docs/architecture.md`), so a
passive observer between two peers cannot read a DM. What Noise does **not**
cover is anything that stores or relays the body in the clear: the receiving
node's database, a future offline-delivery/relay path, or a backup. This lane
adds a second, payload-level envelope sealed for the recipient's device, so the
plaintext never leaves the sender's process.

Whether that is worth the complexity is an open question (see below) — the
lane was started on the "relay/offline delivery may come later" assumption, and
no such path exists yet.

## What is in the tree today

- `src/e2e.rs` — the crypto module:
  - `device_secret` / `peer_messaging_key` / `can_encrypt_to` /
    `can_encrypt_to_str`: derive a static X25519 key from the install's
    Ed25519 identity (standard `ed25519-dalek` conversion), and recover the
    recipient's key from its Ed25519 peer ID without a handshake.
  - `seal_body` / `open_body` / `seal_dm_payload`: per-message ephemeral
    X25519 + HKDF-SHA256 + ChaCha20-Poly1305, with the `msg_id` bound into
    the AAD. The sealed body (`SealedDmBody`) carries content, nickname and
    attachment; only routing fields stay readable.
  - `mark_encrypted`: prefixes the TUI's `[e2e] ` marker onto a rendered line.
  - `ENVELOPE_VERSION`, `HKDF_INFO`, `TUI_ENCRYPTED_MARKER` constants.
  - Unit tests in `tests/unit/unit_e2e.rs`.
- `behavior::DirectMessage.encrypted: Option<EncryptedDm>` — serde-default,
  so older peers that never send it keep working.
- `types::MessageEvent.encrypted: bool` — surfaced to the frontends.
- `messages` table: `encrypted INTEGER NOT NULL DEFAULT 0` column (migration
  `2026-11-02-000000_e2e_encrypted`), `MessageMeta.encrypted`,
  `ChatMessage.encrypted` (mobile).
- `src/frb_generated.rs` regenerated for the new `ChatMessage` field.
- TUI: incoming/outgoing DM lines are prefixed with `[e2e] ` when the flag is
  set (the flag is currently always false, so the marker never appears yet).

## What is deliberately not encrypted

- Broadcasts and group-topic messages (multi-recipient; no per-recipient key).
- Control traffic on the DM lane (acks, nickname exchange) — no user content.
- Local storage: the DB keeps the plaintext (this device is the intended
  reader) plus the `encrypted` flag.

## What is still missing (the wiring)

1. `swarm_handler::send_dm` does **not** call `e2e::seal_dm_payload`; it sends
   plaintext and sets `encrypted: None`.
2. `handle_request_response` does **not** open `request.encrypted`; it never
   calls `e2e::open_incoming`.
3. The mobile `save_incoming_message` path does not receive or persist a real
   flag (it is hardcoded `false`).
4. Flutter has no lock marker in `MessageBubble` (the `ChatMessage.encrypted`
   field exists but is always false).
5. `docs/e2e_encryption.md` is new; `architecture.md` still says there is no
   E2E layer.

## The open question

libp2p Noise already gives transport encryption, and there is no relay or
offline-delivery path today, so the only thing this lane would protect against
is a node reading its own database — which the receiving device is allowed to
do. Completing the wiring is roughly a day of careful work across the DM lane,
both frontends and the web client, plus key-rotation/forward-secrecy questions
that are not answered here (the current scheme has no forward secrecy and no
post-compromise security). Revisit before investing if offline delivery or a
relay is added.
