//! Identity lane (§11 of the backlog; design doc 2026-09-21). Typed DB API for
//! the three identity-lane tables (`users`, `device_bindings`, `contacts`).
//!
//! Hostile-device invariant (enforced here, not trusted): a `device_bindings`
//! row only exists after its signed claim verifies against **that user's**
//! stored account pubkey. A device that lacks a foreign account key can never
//! mint a binding into a foreign user's device-set, because it cannot produce a
//! claim that verifies against that foreign account key. `save_device_binding`
//! refuses to insert unless `verify_device_binding_claim` succeeds first.

use crate::generated::models_insertable::{NewContact, NewDeviceBinding, NewUser};
use crate::generated::models_queryable::{Contact, DeviceBinding, User};
use crate::generated::schema::{contacts, device_bindings, identities, users};
use color_eyre::eyre::WrapErr;
use diesel::prelude::*;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};

/// Persist a stable account (root of trust). Idempotent on `user_id`: an
/// existing account keeps its `account_pubkey` (the root never changes);
/// only the self-username/nickname seams may be refreshed.
///
/// # Errors
/// Returns an error if the insert/update fails.
pub fn save_user(
    user_id: &str,
    account_pubkey: &[u8],
    self_username: Option<&str>,
    self_nickname: Option<&str>,
) -> color_eyre::Result<User> {
    let conn = &mut crate::sqlite_connect()?;
    let self_username = self_username.map(str::to_string);
    let self_nickname = self_nickname.map(str::to_string);
    let new_user = NewUser {
        user_id: user_id.to_string(),
        account_pubkey: account_pubkey.to_vec(),
        self_username: self_username.clone(),
        self_nickname: self_nickname.clone(),
    };
    diesel::insert_into(users::table)
        .values(&new_user)
        .on_conflict(users::user_id)
        .do_update()
        .set((
            users::self_username.eq(self_username),
            users::self_nickname.eq(self_nickname),
        ))
        .returning(User::as_returning())
        .get_result(conn)
        .wrap_err("failed to save user")
}

/// Look up an account by stable `user_id`.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn find_user(user_id: &str) -> color_eyre::Result<Option<User>> {
    let conn = &mut crate::sqlite_connect()?;
    users::table
        .filter(users::user_id.eq(user_id))
        .select(User::as_select())
        .first(conn)
        .optional()
        .wrap_err("failed to look up user")
}

/// Bulk lookup: map several `user_id`s to their full account rows in one round
/// trip.
///
/// Feeds the canonical contact-roster display-name resolution
/// (`contacts::load_contacts`), which needs each grouped user's
/// `self_nickname`/`self_username` but would otherwise pay one query per user.
///
/// # Errors
/// Returns an error if the database query fails.
#[allow(clippy::implicit_hasher)]
pub fn find_users(
    user_ids: &[String],
) -> color_eyre::Result<std::collections::HashMap<String, User>> {
    let conn = &mut crate::sqlite_connect()?;
    users::table
        .filter(users::user_id.eq_any(user_ids))
        .select(User::as_select())
        .load::<User>(conn)
        .map(|rows| rows.into_iter().map(|u| (u.user_id.clone(), u)).collect())
        .wrap_err("failed to look up users")
}

/// The exact bytes a device-binding claim signs over: `user_id ‖
/// device_pubkey ‖ claim_id`.
///
/// Both the verifier ([`verify_device_binding_claim`]) and the minters
/// ([`provision_my_user`], [`mint_device_binding`]) build the message through
/// this single helper, so signing and verification can never diverge byte-for-
/// byte. Note the committed layout does **not** include `bound_at` (the design
/// doc lists it, but the published verify + tests don't sign it; `bound_at` is
/// carried for display and ordering only).
#[must_use]
pub fn device_binding_claim_message(
    user_id: &str,
    device_pubkey: &[u8],
    claim_id: &str,
) -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(user_id.as_bytes());
    message.extend_from_slice(device_pubkey);
    message.extend_from_slice(claim_id.as_bytes());
    message
}

/// Verify a signed device-binding claim against the target user's account key.
///
/// The claim signs `user_id ‖ device_pubkey ‖ claim_id` ([`device_binding_claim_message`]).
/// Verification only succeeds when the signature checks out against the account
/// pubkey stored on that user's row — the byte that enforces the hostile-device
/// invariant.
///
/// # Errors
/// Returns an error if the user is unknown or verification fails.
pub fn verify_device_binding_claim(
    user_id: &str,
    device_pubkey: &[u8],
    claim_id: &str,
    claim: &[u8],
) -> color_eyre::Result<bool> {
    let Some(user) = find_user(user_id)? else {
        return Ok(false);
    };
    let account_pubkey = libp2p_identity::PublicKey::try_decode_protobuf(&user.account_pubkey)
        .wrap_err("stored account pubkey is not a valid libp2p key")?;
    Ok(account_pubkey.verify(
        &device_binding_claim_message(user_id, device_pubkey, claim_id),
        claim,
    ))
}

/// Bind a device into a user's device-set, **only after its claim verifies**
/// against that user's account key (hostile-device invariant).
///
/// # Errors
/// Returns an error if the claim fails verification or the insert fails.
pub fn save_device_binding(
    user_id: &str,
    peer_id: &str,
    device_pubkey: &[u8],
    claim_id: &str,
    claim: &[u8],
) -> color_eyre::Result<DeviceBinding> {
    if !verify_device_binding_claim(user_id, device_pubkey, claim_id, claim)? {
        return Err(color_eyre::eyre::eyre!(
            "device-binding claim failed verification for user {user_id}"
        ));
    }
    let conn = &mut crate::sqlite_connect()?;
    let new_binding = NewDeviceBinding {
        claim_id: claim_id.to_string(),
        user_id: user_id.to_string(),
        peer_id: peer_id.to_string(),
        device_pubkey: device_pubkey.to_vec(),
        claim: claim.to_vec(),
    };
    diesel::insert_into(device_bindings::table)
        .values(&new_binding)
        .returning(DeviceBinding::as_returning())
        .get_result(conn)
        .wrap_err("failed to save device binding")
}

/// List live (unrevoked) devices bound to a user, most recent first.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn list_user_devices(user_id: &str) -> color_eyre::Result<Vec<DeviceBinding>> {
    let conn = &mut crate::sqlite_connect()?;
    device_bindings::table
        .filter(device_bindings::user_id.eq(user_id))
        .filter(device_bindings::revoked_at.is_null())
        .order_by(device_bindings::bound_at.desc())
        .select(DeviceBinding::as_select())
        .load(conn)
        .wrap_err("failed to list user devices")
}

/// Resolve the user bound to a given device peer ID.
///
/// This is the reverse edge of [`list_user_devices`]: given the peer ID of a
/// bound device, return the user it belongs to. Returns `None` when the peer
/// is not bound to any user.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn find_user_id_for_peer(peer_id: &str) -> color_eyre::Result<Option<String>> {
    let conn = &mut crate::sqlite_connect()?;
    device_bindings::table
        .filter(device_bindings::peer_id.eq(peer_id))
        .filter(device_bindings::revoked_at.is_null())
        .select(device_bindings::user_id)
        .first(conn)
        .optional()
        .wrap_err("failed to resolve user for peer")
}

/// Bulk reverse lookup: map each live-bound device peer ID to its user in one
/// round trip.
///
/// Feeds the canonical contact-roster grouping (`contacts::load_contacts`),
/// which would otherwise pay one query per peer. Peers with no live binding
/// (or with no `device_bindings` row at all) are simply absent from the map.
///
/// # Errors
/// Returns an error if the database query fails.
#[allow(clippy::implicit_hasher)]
pub fn find_user_ids_for_peers(
    peer_ids: &[String],
) -> color_eyre::Result<std::collections::HashMap<String, String>> {
    let conn = &mut crate::sqlite_connect()?;
    device_bindings::table
        .filter(device_bindings::peer_id.eq_any(peer_ids))
        .filter(device_bindings::revoked_at.is_null())
        .select((device_bindings::peer_id, device_bindings::user_id))
        .load::<(String, String)>(conn)
        .map(|rows| rows.into_iter().collect())
        .wrap_err("failed to resolve users for peers")
}

/// Add a contact (owner → contact adjacency). Idempotent.
///
/// # Errors
/// Returns an error if the insert fails.
pub fn add_contact(
    owner_user_id: &str,
    contact_user_id: &str,
    nickname_override: Option<&str>,
) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    let new_contact = NewContact {
        owner_user_id: owner_user_id.to_string(),
        contact_user_id: contact_user_id.to_string(),
        nickname_override: nickname_override.map(str::to_string),
    };
    diesel::insert_into(contacts::table)
        .values(&new_contact)
        .on_conflict((contacts::owner_user_id, contacts::contact_user_id))
        .do_nothing()
        .execute(conn)
        .wrap_err("failed to add contact")?;
    Ok(())
}

/// List a user's contacts.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn list_contacts(owner_user_id: &str) -> color_eyre::Result<Vec<Contact>> {
    let conn = &mut crate::sqlite_connect()?;
    contacts::table
        .filter(contacts::owner_user_id.eq(owner_user_id))
        .order_by(contacts::added_at.asc())
        .select(Contact::as_select())
        .load(conn)
        .wrap_err("failed to list contacts")
}

/// Remove a contact (owner → contact adjacency). No-op if absent.
///
/// # Errors
/// Returns an error if the delete fails.
pub fn remove_contact(owner_user_id: &str, contact_user_id: &str) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    diesel::delete(
        contacts::table.filter(
            contacts::owner_user_id
                .eq(owner_user_id)
                .and(contacts::contact_user_id.eq(contact_user_id)),
        ),
    )
    .execute(conn)
    .wrap_err("failed to remove contact")?;
    Ok(())
}

/// Derive the canonical account `user_id` from the account public key.
///
/// The `user_id` is the peer ID of the key itself; nodes that adopt the same
/// account key converge on the same `user_id`, which is what makes the account
/// portable across devices.
#[must_use]
pub fn user_id_from_account_pubkey(pubkey: &libp2p_identity::PublicKey) -> String {
    libp2p::PeerId::from_public_key(pubkey).to_string()
}

/// Persist the account keypair that signs this install's device-binding claims.
///
/// The key is stored on the `identities` row's `account_key` column (NULL until
/// this install provisions a user). Retaining the private account key is a
/// deliberate deviation from the design doc's "identities unchanged" (§11):
/// sibling-device claims can only be minted by the account holder, so the
/// provisioning, self-binding, and accepting flows all need it locally. It
/// unlocks display of the peer id → account mapping but never leaves the
/// device.
///
/// # Errors
/// Returns an error if the identity row cannot be updated or the key cannot be
/// encoded.
pub fn store_account_key(account_key: &libp2p_identity::Keypair) -> color_eyre::Result<()> {
    let encoded = account_key
        .to_protobuf_encoding()
        .wrap_err("failed to encode account key")?;
    let conn = &mut crate::sqlite_connect()?;
    diesel::update(identities::table)
        .set(identities::account_key.eq(Some(encoded)))
        .execute(conn)
        .wrap_err("failed to store account key")?;
    Ok(())
}

/// Load the stored account keypair, if this install has provisioned a user.
///
/// # Errors
/// Returns an error if the identity row cannot be read or the stored key is
/// invalid.
pub fn get_stored_account_key() -> color_eyre::Result<Option<libp2p_identity::Keypair>> {
    let conn = &mut crate::sqlite_connect()?;
    let encoded: Option<Vec<u8>> = identities::table
        .select(identities::account_key)
        .first(conn)
        .optional()
        .wrap_err("failed to read stored account key")?
        .flatten();
    match encoded {
        Some(bytes) => Ok(Some(
            libp2p_identity::Keypair::from_protobuf_encoding(&bytes)
                .wrap_err("stored account key is invalid")?,
        )),
        None => Ok(None),
    }
}

/// Sign a device-binding claim over `user_id ‖ device_pubkey ‖ claim_id` with
/// the account keypair ([`device_binding_claim_message`]).
///
/// # Errors
/// Returns an error if signing fails.
pub fn sign_device_binding_claim(
    account_key: &libp2p_identity::Keypair,
    user_id: &str,
    device_pubkey: &[u8],
    claim_id: &str,
) -> color_eyre::Result<Vec<u8>> {
    account_key
        .sign(&device_binding_claim_message(
            user_id,
            device_pubkey,
            claim_id,
        ))
        .wrap_err("failed to sign device-binding claim")
}

/// Provision this install as a user.
///
/// Mints a fresh account keypair, derives the root `user_id` from it, persists
/// the account, stores the account key, and signs our own first device-binding
/// claim so the install's peer ID is bound into its own user's device-set.
///
/// Idempotent: if this install is already bound into a user, returns that
/// existing `user_id` without touching anything.
///
/// # Errors
/// Returns an error if any persistence or signing step fails.
pub fn provision_my_user(
    self_username: Option<&str>,
    self_nickname: Option<&str>,
) -> color_eyre::Result<String> {
    let peer_id = crate::get_local_peer_id()?.to_string();
    if let Some(user_id) = find_user_id_for_peer(&peer_id)? {
        return Ok(user_id);
    }
    let account_key = libp2p_identity::Keypair::generate_ed25519();
    let user_id = user_id_from_account_pubkey(&account_key.public());
    save_user(
        &user_id,
        &account_key.public().encode_protobuf(),
        self_username,
        self_nickname,
    )?;
    store_account_key(&account_key)?;
    // The device-binding FK requires the peer row to exist: ensure ours does.
    crate::peers::save_peer(&peer_id, &[])?;
    let device_pubkey = crate::get_libp2p_identity()?.public().encode_protobuf();
    let claim_id = crate::gen_msg_id();
    let claim = sign_device_binding_claim(&account_key, &user_id, &device_pubkey, &claim_id)?;
    save_device_binding(&user_id, &peer_id, &device_pubkey, &claim_id, &claim)?;
    Ok(user_id)
}

/// Build the `intro` request that advertises this device and its account.
///
/// The `intro` lets an account holder decide whether to mint a claim for the
/// device. It never requires prior provisioning — an unprovisioned device can
/// still be introduced and later bound by the account holder's explicit accept.
///
/// # Errors
/// Returns an error if the local peer ID or keypair cannot be loaded.
pub fn intro_device_binding_request() -> color_eyre::Result<crate::behavior::DeviceBindingRequest> {
    let peer_id = crate::get_local_peer_id()?.to_string();
    let device_pubkey = crate::get_libp2p_identity()?.public().encode_protobuf();
    let (user_id, self_username, self_nickname) = match find_user_id_for_peer(&peer_id)? {
        Some(uid) => match find_user(&uid)? {
            Some(user) => (
                Some(uid),
                user.self_username.as_deref().map(str::to_string),
                user.self_nickname.as_deref().map(str::to_string),
            ),
            None => (Some(uid), None, None),
        },
        None => (None, None, None),
    };
    Ok(crate::behavior::DeviceBindingRequest {
        kind: "intro".to_string(),
        user_id,
        account_pubkey: None,
        self_username,
        self_nickname,
        peer_id,
        device_pubkey,
        claim_id: None,
        bound_at: None,
        signature: None,
    })
}

/// Mint a signed device-binding claim for a device into **our** account.
///
/// The claim is persisted locally (so our own Contacts collapse that peer into
/// our user immediately), and returned ready to send to the device.
///
/// Requires this install to be provisioned (account key present). The claim is
/// signed over `user_id ‖ device_pubkey ‖ claim_id` and will only be accepted
/// by the target device — which stores it only after re-verifying against our
/// account pubkey.
///
/// # Errors
/// Returns an error if this install has no user/account key, the claim cannot
/// be signed or stored, or the local binding insert fails.
pub fn mint_device_binding(
    peer_id: &str,
    device_pubkey: &[u8],
) -> color_eyre::Result<crate::behavior::DeviceBindingRequest> {
    let local_peer = crate::get_local_peer_id()?.to_string();
    let Some(user_id) = find_user_id_for_peer(&local_peer)? else {
        return Err(color_eyre::eyre::eyre!(
            "this install has no user yet; provision one first"
        ));
    };
    let Some(account_key) = get_stored_account_key()? else {
        return Err(color_eyre::eyre::eyre!(
            "account key missing; re-provision this install"
        ));
    };
    let Some(user) = find_user(&user_id)? else {
        return Err(color_eyre::eyre::eyre!(
            "user {user_id} missing its account row"
        ));
    };
    let claim_id = crate::gen_msg_id();
    let bound_at = crate::current_timestamp();
    let claim = sign_device_binding_claim(&account_key, &user_id, device_pubkey, &claim_id)?;
    // The device-binding FK requires the peer row to exist: ensure the target
    // does (it normally does — it sent us the intro — but be safe).
    crate::peers::save_peer(peer_id, &[])?;
    save_device_binding(&user_id, peer_id, device_pubkey, &claim_id, &claim)?;
    Ok(crate::behavior::DeviceBindingRequest {
        kind: "claim".to_string(),
        user_id: Some(user_id),
        account_pubkey: Some(account_key.public().encode_protobuf()),
        self_username: user.self_username,
        self_nickname: user.self_nickname,
        peer_id: peer_id.to_string(),
        device_pubkey: device_pubkey.to_vec(),
        claim_id: Some(claim_id),
        bound_at: Some(bound_at),
        signature: Some(claim),
    })
}

#[cfg(test)]
#[path = "../tests/unit/unit_identity.rs"]
mod tests;
