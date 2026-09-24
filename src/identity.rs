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
use crate::generated::schema::{contacts, device_bindings, users};
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

/// Verify a signed device-binding claim against the target user's account key.
///
/// The claim signs `user_id ‖ device_pubkey ‖ claim_id`. Verification only
/// succeeds when the signature checks out against the account pubkey stored on
/// that user's row — the byte that enforces the hostile-device invariant.
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
    let mut message = Vec::new();
    message.extend_from_slice(user_id.as_bytes());
    message.extend_from_slice(device_pubkey);
    message.extend_from_slice(claim_id.as_bytes());
    Ok(account_pubkey.verify(&message, claim))
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

#[cfg(test)]
#[path = "../tests/unit/unit_identity.rs"]
mod tests;
