//! End-to-end encryption for direct messages (backlog US-042).
//!
//! ## Threat model
//!
//! The libp2p transports (Noise over TCP/QUIC) already encrypt each hop, so a
//! passive observer *between* two peers cannot read a DM. What that does **not**
//! cover is anywhere the message is stored or relayed in the clear, and it does
//! not survive a future relay/offline-delivery path. This lane adds a second,
//! payload-level envelope: the body is encrypted for the recipient's **device**
//! and the plaintext never leaves the sender's process.
//!
//! ## Which key
//!
//! Encryption keys are derived from the install's Ed25519 **identity** key (the
//! same keypair that produces the peer ID), not from the account key in
//! `users.account_pubkey`:
//!
//! * the account key is the *signing root of trust* for device binding, and its
//!   private half lives only on the account holder's install;
//! * a sibling device bound into the same account (see [`crate::identity`])
//!   does **not** hold the account private key, so account-key encryption would
//!   make every DM to a sibling device undecryptable.
//!
//! Using the identity key means "encrypted to the peer we are talking to",
//! which is exactly the transport-authenticated peer libp2p already gives us.
//! The Ed25519→X25519 conversion is the standard one from `ed25519-dalek`
//! (`SigningKey::to_scalar_bytes` / `VerifyingKey::to_montgomery`).
//!
//! ## Deriving the recipient's key without a handshake
//!
//! An Ed25519 libp2p peer ID is an *identity* multihash whose digest is the
//! protobuf-encoded public key, so the recipient's Ed25519 key is recoverable
//! from the peer ID alone ([`peer_ed25519_public_key`]). No new round trip, no
//! key-directory lookup, and no way to get the wrong key: the peer ID is
//! authenticated by the transport. Peers whose identity is not Ed25519 (RSA,
//! secp256k1, ECDSA) are not convertible; [`can_encrypt_to`] reports that and
//! the caller falls back to plaintext.
//!
//! ## Envelope
//!
//! Per message: a fresh X25519 ephemeral keypair, ECDH against the recipient's
//! static key, HKDF-SHA256 (salt = `ephemeral_pub ‖ recipient_pub`, info =
//! [`HKDF_INFO`]) to a 32-byte ChaCha20-Poly1305 key, a random 12-byte nonce,
//! and AAD = [`HKDF_INFO`] `‖ 0x00 ‖ msg_id`. Binding the message id in the AAD
//! stops an envelope from being replayed under a different `msg_id` (which
//! would otherwise collide with, or bypass, the receipt/dedup bookkeeping).
//!
//! The sealed payload is a [`SealedDmBody`]: the content, the sender nickname
//! and the attachment descriptor all move inside the ciphertext, so only the
//! routing fields (`msg_id`, `sent_at`, `group_id`) stay readable on the wire.
//!
//! ## What is deliberately *not* encrypted
//!
//! * Broadcasts and group-topic messages (multi-recipient; gossipsub has no
//!   per-recipient key and the topic itself is public).
//! * Control traffic on the DM lane: delivery acks and the nickname-only
//!   exchange sent on connect carry no user content, and keeping them in the
//!   clear preserves interoperability with peers that predate this lane.
//! * Local storage: the DB keeps the plaintext (the receiving device is the
//!   intended reader) plus an `encrypted` flag so the frontends can keep
//!   showing that the message *arrived* sealed. See
//!   `docs/e2e_encryption.md`.

use crate::files::MessageAttachment;
use chacha20poly1305::aead::{Aead as _, Payload};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit as _, Nonce};
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use x25519_dalek::{PublicKey, StaticSecret};

/// Version of the sealed-envelope format.
pub const ENVELOPE_VERSION: u8 = 1;

/// HKDF `info` string; also the AAD domain separator prefix.
pub const HKDF_INFO: &[u8] = b"p2p-chat/dm-e2e/v1";

/// Prefix the TUI puts in front of an end-to-end encrypted DM line.
pub const TUI_ENCRYPTED_MARKER: &str = "[e2e] ";

/// Length of an X25519 public key / shared secret, in bytes.
const KEY_LEN: usize = 32;

/// Length of a ChaCha20-Poly1305 nonce, in bytes.
const NONCE_LEN: usize = 12;

/// Length of a ChaCha20-Poly1305 key, in bytes.
const AEAD_KEY_LEN: usize = 32;

/// One sealed DM body, carried in `DirectMessage.encrypted`.
///
/// Bytes are hex-encoded so the envelope stays JSON-friendly (the DM lane uses
/// the JSON codec) and greppable in logs without a base64 dependency.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EncryptedDm {
    /// Envelope version ([`ENVELOPE_VERSION`]); unknown versions are rejected.
    pub version: u8,
    /// Hex-encoded X25519 ephemeral public key used for this message.
    pub ephemeral_key: String,
    /// Hex-encoded 12-byte ChaCha20-Poly1305 nonce.
    pub nonce: String,
    /// Hex-encoded ChaCha20-Poly1305 ciphertext (includes the Poly1305 tag).
    pub ciphertext: String,
}

/// The plaintext that travels inside an [`EncryptedDm`].
///
/// Encrypting the nickname and attachment alongside the content is the point of
/// the lane: an observer that sees the envelope learns the routing fields only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SealedDmBody {
    /// The message text (may be empty for an attachment-only message).
    pub content: String,
    /// Sender nickname, if the sender announced one for this message.
    pub nickname: Option<String>,
    /// Attachment descriptor (files lane §12); the bytes are pulled separately.
    pub attachment: Option<MessageAttachment>,
}

/// The local device's static X25519 secret, derived from its Ed25519 identity.
///
/// Returns `None` when the identity is not Ed25519 (the swarm always provisions
/// an Ed25519 identity, so this is a defensive branch rather than a real state).
#[must_use]
pub fn device_secret(keypair: &libp2p_identity::Keypair) -> Option<StaticSecret> {
    let ed25519 = keypair.clone().try_into_ed25519().ok()?;
    let secret = ed25519.secret();
    let seed: [u8; KEY_LEN] = secret.as_ref().try_into().ok()?;
    let signing = ed25519_dalek::SigningKey::from_bytes(&seed);
    Some(StaticSecret::from(signing.to_scalar_bytes()))
}

/// Recover the Ed25519 public key embedded in an identity-multihash peer ID.
///
/// Ed25519 peer IDs are `<0x00 identity-code> <len> <protobuf PublicKey>`, so
/// the digest *is* the public key. Peer IDs built from a hashed (SHA-256) key —
/// RSA, or any oversized key — are not recoverable and yield `None`.
#[must_use]
pub fn peer_ed25519_public_key(peer_id: &libp2p::PeerId) -> Option<[u8; KEY_LEN]> {
    let multihash = peer_id.as_ref();
    // 0x00 = identity multihash: the digest is the encoded key itself.
    if multihash.code() != 0 {
        return None;
    }
    let public = libp2p_identity::PublicKey::try_decode_protobuf(multihash.digest()).ok()?;
    Some(public.try_into_ed25519().ok()?.to_bytes())
}

/// Derive the recipient's static X25519 public key from its peer ID.
#[must_use]
pub fn peer_messaging_key(peer_id: &libp2p::PeerId) -> Option<PublicKey> {
    let ed25519 = peer_ed25519_public_key(peer_id)?;
    let verifying = ed25519_dalek::VerifyingKey::from_bytes(&ed25519).ok()?;
    Some(PublicKey::from(verifying.to_montgomery().to_bytes()))
}

/// Whether a DM to `peer_id` can be sealed (the peer ID embeds an Ed25519 key).
#[must_use]
pub fn can_encrypt_to(peer_id: &libp2p::PeerId) -> bool {
    peer_messaging_key(peer_id).is_some()
}

/// [`can_encrypt_to`] for a peer ID in the string form the DB and wire carry.
#[must_use]
pub fn can_encrypt_to_str(peer_id: &str) -> bool {
    peer_id
        .parse::<libp2p::PeerId>()
        .is_ok_and(|p| can_encrypt_to(&p))
}

/// Split an outgoing DM into its wire fields, sealing the body when possible.
///
/// Returns `(content, nickname, attachment, envelope)`. When the envelope is
/// `Some`, the three plaintext fields are emptied — the receiver must open the
/// envelope instead, and must not read the empty content as a control message.
///
/// Control traffic deliberately stays in the clear:
///
/// * delivery acks (`ack_for` set) carry no user content, and a peer must be
///   able to answer a message it could not open;
/// * the nickname-only exchange (empty content, no attachment) is how a peer
///   announces its name on connect, and every peer must understand it.
///
/// A peer whose identity is not Ed25519, or a sealing failure, also falls back
/// to plaintext so the lane keeps working with builds that predate it.
#[must_use]
pub fn seal_dm_payload(
    content: String,
    nickname: Option<String>,
    attachment: Option<MessageAttachment>,
    ack_for: Option<&str>,
    msg_id: Option<&str>,
    peer: &libp2p::PeerId,
) -> (
    String,
    Option<String>,
    Option<MessageAttachment>,
    Option<EncryptedDm>,
) {
    let is_control = ack_for.is_some() || (content.trim().is_empty() && attachment.is_none());
    if is_control {
        return (content, nickname, attachment, None);
    }
    let Some(recipient) = peer_messaging_key(peer) else {
        return (content, nickname, attachment, None);
    };
    let body = SealedDmBody {
        content,
        nickname,
        attachment,
    };
    match seal_body(&body, &recipient, msg_id) {
        Ok(envelope) => (String::new(), None, None, Some(envelope)),
        Err(e) => {
            crate::p2plog_debug(format!("dm-e2e: falling back to plaintext: {e}"));
            (body.content, body.nickname, body.attachment, None)
        }
    }
}

/// Prefix [`TUI_ENCRYPTED_MARKER`] onto a rendered body when it arrived sealed.
///
/// The TUI keeps formatted message lines, not message structs, so the marker is
/// folded into the line at both render points (live event and DB reload).
#[must_use]
pub fn mark_encrypted(body: String, encrypted: bool) -> String {
    if encrypted {
        format!("{TUI_ENCRYPTED_MARKER}{body}")
    } else {
        body
    }
}

/// Build the AAD for an envelope: `HKDF_INFO ‖ 0x00 ‖ msg_id`.
fn aad(msg_id: Option<&str>) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        HKDF_INFO
            .len()
            .saturating_add(1)
            .saturating_add(msg_id.map_or(0, str::len)),
    );
    out.extend_from_slice(HKDF_INFO);
    out.push(0);
    if let Some(msg_id) = msg_id {
        out.extend_from_slice(msg_id.as_bytes());
    }
    out
}

/// Derive the per-message AEAD key from an ECDH shared secret.
fn message_key(
    shared: &[u8; KEY_LEN],
    ephemeral: &PublicKey,
    recipient: &PublicKey,
) -> color_eyre::Result<[u8; AEAD_KEY_LEN]> {
    let mut salt = Vec::with_capacity(KEY_LEN.saturating_mul(2));
    salt.extend_from_slice(ephemeral.as_bytes());
    salt.extend_from_slice(recipient.as_bytes());
    let hkdf = hkdf::Hkdf::<sha2::Sha256>::new(Some(&salt), shared);
    let mut key = [0u8; AEAD_KEY_LEN];
    hkdf.expand(HKDF_INFO, &mut key)
        .map_err(|e| eyre!("dm-e2e: HKDF expand failed: {e}"))?;
    Ok(key)
}

/// Seal `body` for `recipient` using caller-supplied randomness.
///
/// Split out from [`seal_body`] so tests can pin the ephemeral key and nonce.
fn seal_with_randomness(
    body: &SealedDmBody,
    recipient: &PublicKey,
    msg_id: Option<&str>,
    ephemeral_secret: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
) -> color_eyre::Result<EncryptedDm> {
    let ephemeral = StaticSecret::from(*ephemeral_secret);
    let ephemeral_public = PublicKey::from(&ephemeral);
    let shared = ephemeral.diffie_hellman(recipient);
    if !shared.was_contributory() {
        return Err(eyre!("dm-e2e: recipient key is not contributory"));
    }
    let key = message_key(shared.as_bytes(), &ephemeral_public, recipient)?;
    let plaintext = serde_json::to_vec(body)?;
    let ciphertext = ChaCha20Poly1305::new((&key).into())
        .encrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: &plaintext,
                aad: &aad(msg_id),
            },
        )
        .map_err(|_| eyre!("dm-e2e: sealing failed"))?;
    Ok(EncryptedDm {
        version: ENVELOPE_VERSION,
        ephemeral_key: to_hex(ephemeral_public.as_bytes()),
        nonce: to_hex(nonce),
        ciphertext: to_hex(&ciphertext),
    })
}

/// Seal `body` for `recipient`, drawing fresh randomness from the OS.
///
/// # Errors
/// Returns an error when the platform RNG fails or the recipient key is
/// degenerate (a low-order point with no usable shared secret).
pub fn seal_body(
    body: &SealedDmBody,
    recipient: &PublicKey,
    msg_id: Option<&str>,
) -> color_eyre::Result<EncryptedDm> {
    let mut ephemeral = [0u8; KEY_LEN];
    getrandom::fill(&mut ephemeral).map_err(|e| eyre!("dm-e2e: no randomness: {e}"))?;
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).map_err(|e| eyre!("dm-e2e: no randomness: {e}"))?;
    seal_with_randomness(body, recipient, msg_id, &ephemeral, &nonce)
}

/// Open `envelope` with the local device secret.
///
/// # Errors
/// Returns an error for an unknown envelope version, malformed hex, a
/// degenerate ephemeral key, or a failed authentication (tampering, wrong
/// recipient key, or a mismatched `msg_id`).
pub fn open_body(
    envelope: &EncryptedDm,
    secret: &StaticSecret,
    msg_id: Option<&str>,
) -> color_eyre::Result<SealedDmBody> {
    if envelope.version != ENVELOPE_VERSION {
        return Err(eyre!(
            "dm-e2e: unsupported envelope version {}",
            envelope.version
        ));
    }
    let ephemeral_bytes = from_hex(&envelope.ephemeral_key)
        .ok_or_else(|| eyre!("dm-e2e: malformed ephemeral key"))?;
    let ephemeral_array: [u8; KEY_LEN] = ephemeral_bytes
        .as_slice()
        .try_into()
        .map_err(|_| eyre!("dm-e2e: ephemeral key is not {KEY_LEN} bytes"))?;
    let nonce_bytes = from_hex(&envelope.nonce).ok_or_else(|| eyre!("dm-e2e: malformed nonce"))?;
    let nonce_array: [u8; NONCE_LEN] = nonce_bytes
        .as_slice()
        .try_into()
        .map_err(|_| eyre!("dm-e2e: nonce is not {NONCE_LEN} bytes"))?;
    let ciphertext =
        from_hex(&envelope.ciphertext).ok_or_else(|| eyre!("dm-e2e: malformed ciphertext"))?;

    let ephemeral_public = PublicKey::from(ephemeral_array);
    let our_public = PublicKey::from(secret);
    let shared = secret.diffie_hellman(&ephemeral_public);
    if !shared.was_contributory() {
        return Err(eyre!("dm-e2e: sender key is not contributory"));
    }
    let key = message_key(shared.as_bytes(), &ephemeral_public, &our_public)?;
    let plaintext = ChaCha20Poly1305::new((&key).into())
        .decrypt(
            Nonce::from_slice(&nonce_array),
            Payload {
                msg: &ciphertext,
                aad: &aad(msg_id),
            },
        )
        .map_err(|_| eyre!("dm-e2e: authentication failed (tampered or not for us)"))?;
    Ok(serde_json::from_slice(&plaintext)?)
}

/// Seal `body` for the peer named by `peer_id`, when it is encryptable.
///
/// Returns `None` when the peer ID does not embed an Ed25519 key (plaintext
/// fallback) or when the local identity cannot be read.
///
/// # Errors
/// Returns an error only when the peer *is* encryptable but sealing failed; the
/// caller then decides between failing the send and falling back to plaintext.
pub fn seal_for_peer(
    peer_id: &str,
    msg_id: Option<&str>,
    body: &SealedDmBody,
) -> color_eyre::Result<Option<EncryptedDm>> {
    let Ok(peer) = peer_id.parse::<libp2p::PeerId>() else {
        return Ok(None);
    };
    let Some(recipient) = peer_messaging_key(&peer) else {
        return Ok(None);
    };
    seal_body(body, &recipient, msg_id).map(Some)
}

/// Open an inbound sealed DM with this install's device key.
///
/// The sender's identity needs no separate check: the transport authenticated
/// it, and the envelope only opens for the device it was sealed to.
///
/// # Errors
/// Returns an error when the local identity is unavailable or not Ed25519, or
/// when the envelope does not authenticate.
pub fn open_incoming(
    msg_id: Option<&str>,
    envelope: &EncryptedDm,
) -> color_eyre::Result<SealedDmBody> {
    let keypair = crate::db::get_libp2p_identity()?;
    let secret =
        device_secret(&keypair).ok_or_else(|| eyre!("dm-e2e: local identity is not Ed25519"))?;
    open_body(envelope, &secret, msg_id)
}

/// Hex-encode bytes (lowercase, no prefix).
fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        // Writing to a `String` cannot fail.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Decode lowercase or uppercase hex; `None` on any malformed input.
fn from_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() & 1 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(text.len());
    for pair in text.as_bytes().as_chunks::<2>().0 {
        let chunk = std::str::from_utf8(pair).ok()?;
        out.push(u8::from_str_radix(chunk, 16).ok()?);
    }
    Some(out)
}

#[cfg(test)]
#[path = "../tests/unit/unit_e2e.rs"]
mod tests;
