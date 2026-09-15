//! Private-group cryptography: control-plane membership messages.
//!
//! ## Model
//!
//! Private groups are **not** built on a broadcast channel. Both group
//! content and membership control travel over the existing point-to-point DM
//! path (`request_response <DirectMessage, DirectMessage>`), which rides the
//! noise transport. The transport already provides per-recipient X25519 key
//! exchange and ChaCha20-Poly1305 AEAD, so there is no group-level key
//! wrapping, pairwise-secret derivation, or envelope machinery here.
//!
//! Membership is enforced **structurally**: an existing member simply does not
//! send a group DM to a non-member. A joining peer can never grant itself
//! membership (only an existing member sends an invite), and a removed peer
//! stops receiving deliveries from members.
//!
//! What remains here is the *control plane*: membership changes are authorized
//! by an existing member and signed with the node's libp2p Ed25519 identity:
//!
//! * [`GroupControl::AddMember`] is signed by an existing member, so a peer
//!   cannot add itself.
//! * [`GroupControl::RemoveMember`] is signed by an existing member, so a
//!   removed peer cannot remove its membership on its own.
//!
//! [`sign_group_control`] signs using the node's Ed25519 identity keypair;
//! [`verify_group_control`] checks the signature against the acting member's
//! stored Ed25519 public key hex.

use color_eyre::eyre::{Context as _, Result, eyre};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Length of an Ed25519 public key in bytes.
pub const ED25519_KEY_LEN: usize = 32;

/// Length of an Ed25519 signature in bytes.
pub const ED25519_SIG_LEN: usize = 64;

/// A membership-control action for a private group.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupControl {
    /// Invite `target_peer_id` into the group. Must be signed by an existing
    /// member.
    AddMember {
        /// The group this control targets.
        group_id: String,
        /// The member being invited.
        target_peer_id: String,
    },
    /// Remove `target_peer_id` from the group. Must be signed by an existing
    /// member.
    RemoveMember {
        /// The group this control targets.
        group_id: String,
        /// The member being removed.
        target_peer_id: String,
    },
}

/// A [`GroupControl`] signed by an existing member's Ed25519 identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedGroupControl {
    /// The control action.
    pub control: GroupControl,
    /// The acting member's peer id (the signer).
    pub signer: String,
    /// Hex-encoded Ed25519 signature over the serialized control.
    pub signature: String,
}

/// Extract the node's 32-byte Ed25519 public key from its libp2p identity.
///
/// # Errors
/// Returns an error if the identity is not an Ed25519 key.
pub fn ed25519_public_bytes(keypair: &libp2p_identity::Keypair) -> Result<[u8; ED25519_KEY_LEN]> {
    let public = keypair
        .public()
        .try_into_ed25519()
        .map_err(|_| eyre!("libp2p identity is not an ed25519 key"))?;
    Ok(public.to_bytes())
}

/// Sign a membership-control message with the node's Ed25519 identity keypair.
///
/// # Errors
/// Returns an error if signing fails.
pub fn sign_group_control(
    control: GroupControl,
    signer_peer_id: &str,
    keypair: &libp2p_identity::Keypair,
) -> Result<SignedGroupControl> {
    let message = serde_json::to_vec(&control).wrap_err("failed to serialize group control")?;
    let signature = keypair
        .sign(&message)
        .wrap_err("failed to sign group control")?;
    Ok(SignedGroupControl {
        control,
        signer: signer_peer_id.to_string(),
        signature: hex_encode(&signature),
    })
}

/// Verify a signed control message against the acting member's Ed25519 public
/// key hex (from the local member list).
///
/// # Errors
/// Returns an error if the key/signature hex is malformed or the signature
/// does not verify.
pub fn verify_group_control(
    signed: &SignedGroupControl,
    member_ed25519_key_hex: &str,
) -> Result<()> {
    let public_bytes = hex_decode(member_ed25519_key_hex)?;
    let public_array: [u8; ED25519_KEY_LEN] = public_bytes
        .as_slice()
        .try_into()
        .map_err(|_| eyre!("ed25519 key must be {ED25519_KEY_LEN} bytes"))?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_array).wrap_err("malformed ed25519 public key")?;
    let signature_bytes = hex_decode(&signed.signature)?;
    let signature_array: [u8; ED25519_SIG_LEN] = signature_bytes
        .as_slice()
        .try_into()
        .map_err(|_| eyre!("signature must be {ED25519_SIG_LEN} bytes"))?;
    let signature = Signature::from_bytes(&signature_array);
    let message =
        serde_json::to_vec(&signed.control).wrap_err("failed to serialize group control")?;
    verifying_key
        .verify_strict(&message, &signature)
        .wrap_err("group control signature does not verify")
}

/// Stable group address derived from the group's signing keypair.
///
/// Uses the exact `PeerId::from_public_key` machinery that already fans
/// point-to-point DMs (see `src/behavior.rs` `build_swarm`), so a private
/// group is a peer-id-shaped identity *in the DM address space* — no second
/// namespace on the wireWhat else, choosing a fresh peer-id for a group is
/// exactly like minting a DM-capable peer: both endpoints just keep using the
/// same attribution already riding `request_response`.
/// Shared, peer-id-shaped address of a private group.
///
/// Rides the exact [`libp2p::PeerId::from_public_key`] machinery already
/// addressing point-to-point DMs, so a private group lives *in the DM address
/// space* — no second namespace, no new wire semantics. Crucially the input is
/// the **group's public key** (minted once at group creation, stored on the
/// group record, read from the same row by every member), so *every member
/// pair derives the identical `PeerId`*: it is group-wide attribution, not a
/// per-sender handle. The per-sender "who" already rides the DM wire itself.
///
/// # Stability across pairs
/// Two members of the same private group pass the *same* group public key and
/// therefore receive the *same* `PeerId` — the property that makes the tag
/// meaningful on a shared wire.
#[must_use]
pub fn group_peer_id(group_public: &libp2p_identity::PublicKey) -> libp2p::PeerId {
    libp2p::PeerId::from_public_key(group_public)
}

/// Encode bytes as lowercase hex.
#[must_use]
pub fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for b in bytes {
        out.push(char::from_digit(u32::from(b >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(b & 0x0f), 16).unwrap_or('0'));
    }
    out
}

/// Decode lowercase hex into bytes.
///
/// # Errors
/// Returns an error on malformed input (odd length or non-hex characters).
pub fn hex_decode(s: &str) -> Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return Err(eyre!("odd-length hex string"));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for pair in s.as_bytes().chunks_exact(2) {
        let hi = nibble(
            pair.first()
                .copied()
                .ok_or_else(|| eyre!("non-hex character in string"))?,
        )
        .ok_or_else(|| eyre!("non-hex character in string"))?;
        let lo = nibble(
            pair.get(1)
                .copied()
                .ok_or_else(|| eyre!("non-hex character in string"))?,
        )
        .ok_or_else(|| eyre!("non-hex character in string"))?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

/// Value of a hex nibble, or `None` if `b` is not a lowercase hex digit.
#[must_use]
const fn nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b.wrapping_sub(b'0')),
        b'a'..=b'f' => Some(b.wrapping_sub(b'a').wrapping_add(10)),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../tests/unit/unit_group_crypto.rs"]
mod tests;
