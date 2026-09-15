//! Unit tests for the private-group control-plane cryptography module
//! (`src/group_crypto.rs`).
//!
//! The module exposes a small pure-Rust API: lowercase hex helpers, Ed25519
//! public-key extraction, and signed membership-control round-tripping. None
//! of these touch the database, so they follow the plain `use super::*;`
//! convention (no `serial`/`TempDir` machinery).

use super::*;

// ── Hex helpers ─────────────────────────────────────────────────────────────

#[test]
fn hex_encode_is_lowercase_and_doubles_length() {
    assert_eq!(hex_encode(&[0xde, 0xad]), "dead");
    assert_eq!(hex_encode(&[0xbe, 0xef]), "beef");
    assert_eq!(hex_encode(&[0x00, 0x01, 0x7f, 0x80, 0xff]), "00017f80ff");
    assert_eq!(hex_encode(&[]), "");
}

#[test]
fn hex_decode_roundtrips_and_rejects_malformed() {
    let bytes = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x7f, 0x80, 0xff];
    let hex = hex_encode(&bytes);
    assert_eq!(hex_decode(&hex).expect("decode"), bytes);

    assert!(hex_decode("abc").is_err(), "odd length rejected");
    assert!(hex_decode("0").is_err(), "odd length rejected");
    assert!(hex_decode("").is_ok(), "empty is empty");
    assert!(hex_decode("0g").is_err(), "non-hex digit rejected");
    assert!(hex_decode("zz").is_err(), "non-hex rejected");
    assert!(hex_decode("0A").is_err(), "uppercase rejected");
}

#[test]
fn nibble_maps_lowercase_hex_digits() {
    assert_eq!(nibble(b'0'), Some(0));
    assert_eq!(nibble(b'9'), Some(9));
    assert_eq!(nibble(b'a'), Some(10));
    assert_eq!(nibble(b'f'), Some(15));
    assert_eq!(nibble(b'A'), None);
    assert_eq!(nibble(b'F'), None);
    assert_eq!(nibble(b'g'), None);
    assert_eq!(nibble(b' '), None);
}

// ── Ed25519 public-key extraction ───────────────────────────────────────────

#[test]
fn ed25519_public_bytes_returns_32_byte_ed25519_key() {
    let keypair = libp2p_identity::Keypair::generate_ed25519();
    let public = ed25519_public_bytes(&keypair).expect("public bytes");
    assert_eq!(public.len(), 32, "key length");
    assert_eq!(hex_encode(&public).len(), 32 * 2, "hex length");
}

// ── Sign / verify membership control ────────────────────────────────────────

#[test]
fn sign_then_verify_accepts_genuine_control() {
    let keypair = libp2p_identity::Keypair::generate_ed25519();
    let public_hex = hex_encode(&ed25519_public_bytes(&keypair).expect("public bytes"));
    let signed = sign_group_control(
        GroupControl::AddMember {
            group_id: "lobby".to_string(),
            target_peer_id: "peer-b".to_string(),
        },
        "peer-a",
        &keypair,
    )
    .expect("sign");
    assert_eq!(signed.signer, "peer-a");
    verify_group_control(&signed, &public_hex).expect("verify");
    assert!(
        verify_group_control(&signed, &hex_encode(&[0u8; 32])).is_err(),
        "public key mismatch rejected"
    );
    assert!(
        verify_group_control(&signed, &hex_encode(&[0xf4u8; 2])).is_err(),
        "unparsable key rejected"
    );
}

#[test]
fn tampered_control_fails_verification() {
    let keypair = libp2p_identity::Keypair::generate_ed25519();
    let public_hex = hex_encode(&ed25519_public_bytes(&keypair).expect("public bytes"));
    let mut signed = sign_group_control(
        GroupControl::RemoveMember {
            group_id: "squad".to_string(),
            target_peer_id: "peer-b".to_string(),
        },
        "peer-a",
        &keypair,
    )
    .expect("sign");
    if let GroupControl::RemoveMember { target_peer_id, .. } = &mut signed.control {
        *target_peer_id = "peer-c".to_string();
    }
    assert!(
        verify_group_control(&signed, &public_hex).is_err(),
        "tampered control must be rejected"
    );
}

#[test]
fn control_signed_by_wrong_key_fails_verification() {
    let signer_keypair = libp2p_identity::Keypair::generate_ed25519();
    let attacker_keypair = libp2p_identity::Keypair::generate_ed25519();
    let signer_public_hex =
        hex_encode(&ed25519_public_bytes(&signer_keypair).expect("public bytes"));
    let signed = sign_group_control(
        GroupControl::AddMember {
            group_id: "squad".to_string(),
            target_peer_id: "peer-b".to_string(),
        },
        "peer-a",
        &attacker_keypair,
    )
    .expect("sign");
    assert!(
        verify_group_control(&signed, &signer_public_hex).is_err(),
        "signature from another key must be rejected"
    );
}

#[test]
fn group_id_is_shared_across_members_and_distinct_per_group() {
    let group_keypair = libp2p_identity::Keypair::generate_ed25519();
    let other_group_keypair = libp2p_identity::Keypair::generate_ed25519();

    let shared = group_peer_id(&group_keypair.public());
    let shared_again = group_peer_id(&group_keypair.public());

    assert_eq!(shared, shared_again, "the same shared group public key must mint the same group PeerId, no matter which pair derives it");
    assert_eq!(
        shared,
        group_peer_id(&group_keypair.public()),
        "group wide attribution is stable across pairs"
    );
    assert_ne!(
        shared,
        group_peer_id(&other_group_keypair.public()),
        "a different group's shared key must mint a different group PeerId"
    );
}
