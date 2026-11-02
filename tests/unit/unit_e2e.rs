use super::*;

fn body(content: &str) -> SealedDmBody {
    SealedDmBody {
        content: content.to_string(),
        nickname: Some("alice".to_string()),
        attachment: None,
    }
}

fn keypair() -> libp2p_identity::Keypair {
    libp2p_identity::Keypair::generate_ed25519()
}

#[test]
fn hex_round_trips_and_rejects_junk() {
    let bytes = [0u8, 1, 15, 16, 127, 128, 255];
    let encoded = to_hex(&bytes);
    assert_eq!(encoded, "00010f107f80ff");
    assert_eq!(from_hex(&encoded), Some(bytes.to_vec()));
    assert_eq!(from_hex("ABCD"), from_hex("abcd"));
    assert_eq!(from_hex("abc"), None);
    assert_eq!(from_hex("zz"), None);
}

#[test]
fn peer_messaging_key_is_recoverable_from_peer_id() {
    let keypair = keypair();
    let peer_id = keypair.public().to_peer_id();
    let embedded = peer_ed25519_public_key(&peer_id).expect("ed25519 peer id");
    let ed25519 = keypair
        .try_into_ed25519()
        .expect("ed25519 keypair")
        .public()
        .to_bytes();
    assert_eq!(embedded, ed25519);
    assert!(can_encrypt_to(&peer_id));
}

#[test]
fn device_secret_matches_the_derived_public_key() {
    let keypair = keypair();
    let secret = device_secret(&keypair).expect("ed25519 identity");
    let public = PublicKey::from(&secret);
    let expected = peer_messaging_key(&keypair.public().to_peer_id()).expect("derived key");
    assert_eq!(public.as_bytes(), expected.as_bytes());
}

#[test]
fn seal_and_open_round_trip() {
    let sender = keypair();
    let recipient = keypair();
    let recipient_secret = device_secret(&recipient).expect("recipient secret");
    let recipient_public =
        peer_messaging_key(&recipient.public().to_peer_id()).expect("recipient public");

    let envelope = seal_body(&body("hello bob"), &recipient_public, Some("m1")).expect("seal");
    assert_eq!(envelope.version, ENVELOPE_VERSION);
    // Nothing of the plaintext survives in the envelope.
    assert!(!envelope.ciphertext.contains("hello"));
    assert!(envelope.ciphertext.len() > 32);

    let opened = open_body(&envelope, &recipient_secret, Some("m1")).expect("open");
    assert_eq!(opened, body("hello bob"));

    // The sender's own key cannot open it (sealed to the recipient device).
    let sender_secret = device_secret(&sender).expect("sender secret");
    assert!(open_body(&envelope, &sender_secret, Some("m1")).is_err());
}

#[test]
fn attachment_descriptor_survives_the_envelope() {
    let recipient = keypair();
    let recipient_secret = device_secret(&recipient).expect("secret");
    let recipient_public = peer_messaging_key(&recipient.public().to_peer_id()).expect("public");
    let with_file = SealedDmBody {
        content: String::new(),
        nickname: None,
        attachment: Some(MessageAttachment {
            file_id: "abc".to_string(),
            name: "notes.txt".to_string(),
            size: 12,
            mime: None,
        }),
    };
    let envelope = seal_body(&with_file, &recipient_public, None).expect("seal");
    assert_eq!(
        open_body(&envelope, &recipient_secret, None).expect("open"),
        with_file
    );
}

#[test]
fn message_id_is_bound_into_the_aad() {
    let recipient = keypair();
    let recipient_secret = device_secret(&recipient).expect("secret");
    let recipient_public = peer_messaging_key(&recipient.public().to_peer_id()).expect("public");
    let envelope = seal_body(&body("bound"), &recipient_public, Some("m1")).expect("seal");
    assert!(open_body(&envelope, &recipient_secret, Some("m2")).is_err());
    assert!(open_body(&envelope, &recipient_secret, None).is_err());
}

#[test]
fn tampered_ciphertext_is_rejected() {
    let recipient = keypair();
    let recipient_secret = device_secret(&recipient).expect("secret");
    let recipient_public = peer_messaging_key(&recipient.public().to_peer_id()).expect("public");
    let envelope = seal_body(&body("tamper me"), &recipient_public, Some("m1")).expect("seal");

    let mut mutated = envelope.ciphertext.clone();
    let first = mutated.remove(0);
    mutated.insert(0, if first == '0' { '1' } else { '0' });
    let tampered = EncryptedDm {
        ciphertext: mutated,
        ..envelope.clone()
    };
    assert!(open_body(&tampered, &recipient_secret, Some("m1")).is_err());

    let bad_nonce = EncryptedDm {
        nonce: "00".repeat(NONCE_LEN),
        ..envelope
    };
    assert!(open_body(&bad_nonce, &recipient_secret, Some("m1")).is_err());
}

#[test]
fn unknown_version_and_malformed_fields_are_rejected() {
    let recipient = keypair();
    let recipient_secret = device_secret(&recipient).expect("secret");
    let recipient_public = peer_messaging_key(&recipient.public().to_peer_id()).expect("public");
    let envelope = seal_body(&body("v2?"), &recipient_public, None).expect("seal");

    let future = EncryptedDm {
        version: ENVELOPE_VERSION.saturating_add(1),
        ..envelope.clone()
    };
    assert!(open_body(&future, &recipient_secret, None).is_err());
    assert!(open_body(&EncryptedDm::default(), &recipient_secret, None).is_err());

    let short_key = EncryptedDm {
        ephemeral_key: "00".to_string(),
        ..envelope.clone()
    };
    assert!(open_body(&short_key, &recipient_secret, None).is_err());

    let bad_hex = EncryptedDm {
        ciphertext: "not-hex".to_string(),
        ..envelope
    };
    assert!(open_body(&bad_hex, &recipient_secret, None).is_err());
}

#[test]
fn each_message_uses_fresh_randomness() {
    let recipient = keypair();
    let recipient_public = peer_messaging_key(&recipient.public().to_peer_id()).expect("public");
    let first = seal_body(&body("same"), &recipient_public, Some("m1")).expect("seal");
    let second = seal_body(&body("same"), &recipient_public, Some("m1")).expect("seal");
    assert_ne!(first.ephemeral_key, second.ephemeral_key);
    assert_ne!(first.nonce, second.nonce);
    assert_ne!(first.ciphertext, second.ciphertext);
}

#[test]
fn seal_for_peer_needs_an_ed25519_peer_id() {
    let recipient = keypair();
    let peer_id = recipient.public().to_peer_id();
    let sealed = seal_for_peer(&peer_id.to_string(), Some("m1"), &body("hi"))
        .expect("seal")
        .expect("encryptable peer");
    let secret = device_secret(&recipient).expect("secret");
    assert_eq!(
        open_body(&sealed, &secret, Some("m1")).expect("open"),
        body("hi")
    );

    assert!(
        seal_for_peer("not-a-peer-id", None, &body("hi"))
            .expect("fallback")
            .is_none()
    );
}

#[test]
fn deterministic_seal_with_pinned_randomness() {
    let recipient = keypair();
    let recipient_public = peer_messaging_key(&recipient.public().to_peer_id()).expect("public");
    let ephemeral = [7u8; KEY_LEN];
    let nonce = [9u8; NONCE_LEN];
    let first = seal_with_randomness(&body("fixed"), &recipient_public, None, &ephemeral, &nonce)
        .expect("seal");
    let second = seal_with_randomness(&body("fixed"), &recipient_public, None, &ephemeral, &nonce)
        .expect("seal");
    assert_eq!(first, second);
}

#[test]
fn non_ed25519_peer_ids_are_not_encryptable() {
    // A SHA-256 multihash peer ID (what an RSA identity produces) has no
    // recoverable public key, so the caller falls back to plaintext.
    let mut raw = vec![0x12u8, 0x20];
    raw.extend_from_slice(&[3u8; 32]);
    let peer_id = libp2p::PeerId::from_bytes(&raw).expect("sha256 peer id");
    assert_eq!(peer_ed25519_public_key(&peer_id), None);
    assert!(peer_messaging_key(&peer_id).is_none());
    assert!(!can_encrypt_to(&peer_id));
}
