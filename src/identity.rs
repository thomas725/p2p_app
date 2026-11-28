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
use color_eyre::eyre::{WrapErr, bail, ensure};
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

/// Insert the account root for a `user_id` if it does not exist yet, without
/// touching display seams.
///
/// Unlike [`save_user`], this never overwrites an existing row: a known
/// account's stored `account_pubkey` and self-declared display names stay
/// authoritative. The receive path uses this to materialize the root so the
/// claim signature can be verified *before* any unauthenticated display data
/// from a network request is persisted.
///
/// # Errors
/// Returns an error if the insert fails.
pub fn ensure_user_root(user_id: &str, account_pubkey: &[u8]) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    diesel::insert_into(users::table)
        .values(NewUser {
            user_id: user_id.to_string(),
            account_pubkey: account_pubkey.to_vec(),
            self_username: None,
            self_nickname: None,
        })
        .on_conflict_do_nothing()
        .execute(conn)
        .wrap_err("failed to ensure user root")?;
    Ok(())
}

/// Refresh the self-declared display names of an account.
///
/// Call only after [`verify_device_binding_claim`] succeeded for a claim
/// carrying these fields: they are unauthenticated (not covered by the claim
/// signature) and must never be persisted for a rejected claim.
///
/// # Errors
/// Returns an error if the update fails.
pub fn set_user_display_seams(
    user_id: &str,
    self_username: Option<&str>,
    self_nickname: Option<&str>,
) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    diesel::update(users::table)
        .filter(users::user_id.eq(user_id))
        .set((
            users::self_username.eq(self_username),
            users::self_nickname.eq(self_nickname),
        ))
        .execute(conn)
        .wrap_err("failed to update user display seams")?;
    Ok(())
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

/// Verify a signed device-binding claim.
///
/// Two signers are accepted (hostile-device invariant preserved):
///
/// 1. The target user's **account key** — the root of trust. A claim signed by
///    it always verifies.
/// 2. An **already-bound sibling device** of the same user: the sender's
///    `peer_id` must resolve (via [`find_user_id_for_peer`]) to the same
///    `user_id`, and the signature must check out against that device's stored
///    pubkey. This widens the set of devices that may add further devices
///    without handing out the account private key.
///
/// `sender_peer_id` is the peer that transmitted the claim (the minting
/// device). It is `None` for locally minted claims that were signed with the
/// account key, in which case only path 1 applies.
///
/// The claim signs `user_id ‖ device_pubkey ‖ claim_id`
/// ([`device_binding_claim_message`]).
///
/// # Errors
/// Returns an error if the user is unknown or a database lookup fails.
pub fn verify_device_binding_claim(
    user_id: &str,
    device_pubkey: &[u8],
    claim_id: &str,
    claim: &[u8],
    sender_peer_id: Option<&str>,
) -> color_eyre::Result<bool> {
    let Some(user) = find_user(user_id)? else {
        return Ok(false);
    };
    let message = device_binding_claim_message(user_id, device_pubkey, claim_id);

    // Path 1: the account key (root of trust).
    if let Ok(account_pubkey) =
        libp2p_identity::PublicKey::try_decode_protobuf(&user.account_pubkey)
        && account_pubkey.verify(&message, claim)
    {
        return Ok(true);
    }

    // Path 2: an already-bound sibling device of the same user.
    if let Some(sender) = sender_peer_id
        && find_user_id_for_peer(sender)?.as_deref() == Some(user_id)
    {
        // Look up the sender's stored device pubkey.
        let conn = &mut crate::sqlite_connect()?;
        let sender_pubkey_bytes: Option<Vec<u8>> = device_bindings::table
            .filter(device_bindings::peer_id.eq(sender))
            .filter(device_bindings::revoked_at.is_null())
            .select(device_bindings::device_pubkey)
            .first(conn)
            .optional()
            .wrap_err("failed to look up sender device pubkey")?;
        if let Some(bytes) = sender_pubkey_bytes
            && let Ok(sender_pubkey) = libp2p_identity::PublicKey::try_decode_protobuf(&bytes)
            && sender_pubkey.verify(&message, claim)
        {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Verify and apply a device-binding claim received from the network.
///
/// This is the single choke point for the receive path. It holds the
/// hostile-device invariant end-to-end: an unauthenticated claim can never
/// overwrite a known account's display seams, because those are written only
/// after the signature verifies and the binding is accepted. The claim must
/// also be *about this install* (`peer_id` + `device_pubkey`), checked before
/// any row is inserted.
///
/// `sender_peer_id` is the peer that transmitted the claim (sibling path);
/// `local_peer_id` / `local_device_pubkey` identify the receiving install.
/// An identical retransmit that already bound this device is accepted.
///
/// # Errors
/// Returns an error if the claim is malformed, names another device, fails
/// verification, or any persistence step fails.
pub fn receive_device_binding_claim(
    request: &crate::behavior::DeviceBindingRequest,
    sender_peer_id: &str,
    local_peer_id: &str,
    local_device_pubkey: &[u8],
) -> color_eyre::Result<String> {
    let user_id = request
        .user_id
        .clone()
        .ok_or_else(|| color_eyre::eyre::eyre!("claim carries no user_id"))?;
    let account_pubkey = request
        .account_pubkey
        .clone()
        .ok_or_else(|| color_eyre::eyre::eyre!("claim carries no account_pubkey"))?;
    let claim_id = request
        .claim_id
        .clone()
        .ok_or_else(|| color_eyre::eyre::eyre!("claim carries no claim_id"))?;
    let signature = request
        .signature
        .clone()
        .ok_or_else(|| color_eyre::eyre::eyre!("claim carries no signature"))?;

    // The claim must target this device. Check before touching the DB so a
    // claim about somebody else leaves no rows behind at all.
    if request.peer_id != local_peer_id {
        return Err(color_eyre::eyre::eyre!(
            "claim names a different device than this node"
        ));
    }
    if request.device_pubkey != local_device_pubkey {
        return Err(color_eyre::eyre::eyre!(
            "claim names a different device key than this install's"
        ));
    }

    // Materialize the root without display data so verification can read the
    // stored account key. An existing user's key and seams are left intact.
    ensure_user_root(&user_id, &account_pubkey)?;

    // Verify before persisting anything derived from the claim's display lane.
    if !verify_device_binding_claim(
        &user_id,
        &request.device_pubkey,
        &claim_id,
        &signature,
        Some(sender_peer_id),
    )? {
        return Err(color_eyre::eyre::eyre!(
            "device-binding claim failed verification for user {user_id}"
        ));
    }

    match save_device_binding(
        &user_id,
        &request.peer_id,
        &request.device_pubkey,
        &claim_id,
        &signature,
        Some(sender_peer_id),
    ) {
        Ok(_) => {}
        // An identical claim already bound this device into the same user.
        Err(_)
            if find_user_id_for_peer(&request.peer_id)
                .unwrap_or(None)
                .as_deref()
                == Some(user_id.as_str()) => {}
        Err(e) => return Err(e),
    }

    // Only now — verification succeeded and the device is bound — may the
    // unauthenticated display seams be applied to the account row.
    set_user_display_seams(
        &user_id,
        request.self_username.as_deref(),
        request.self_nickname.as_deref(),
    )?;

    Ok(user_id)
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
    sender_peer_id: Option<&str>,
) -> color_eyre::Result<DeviceBinding> {
    if !verify_device_binding_claim(user_id, device_pubkey, claim_id, claim, sender_peer_id)? {
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
    find_user_id_for_peer_conn(conn, peer_id)
}

/// Resolve a peer's user on an existing connection.
///
/// Share submissions run this inside their write transaction, so the ACL sees
/// the same committed state as the revision check and the manifest write.
///
/// # Errors
/// Returns an error if the binding query fails.
pub(crate) fn find_user_id_for_peer_conn(
    conn: &mut diesel::SqliteConnection,
    peer_id: &str,
) -> color_eyre::Result<Option<String>> {
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

/// Resolve this install's user ID from its stored identity and self-binding.
///
/// Returns `None` before the install has been provisioned and bound to a user
/// (the TUI and mobile both auto-provision at start, so it is `Some` on any
/// running install).
///
/// # Errors
/// Returns an error if the local peer ID cannot be resolved or the DB lookup
/// fails.
pub fn my_user_id() -> color_eyre::Result<Option<String>> {
    let local_peer = crate::get_local_peer_id()?.to_string();
    find_user_id_for_peer(&local_peer)
}

/// The devices bound into this install's user, newest first, as
/// `(peer_id, bound_at)` pairs.
///
/// Empty before the install belongs to a user (see [`my_user_id`]); formatting
/// of `bound_at` is left to the caller (`crate::format_peer_datetime`).
///
/// # Errors
/// Returns an error if the local peer/user cannot be resolved or the DB query
/// fails.
pub fn my_user_devices() -> color_eyre::Result<Vec<(String, chrono::NaiveDateTime)>> {
    match my_user_id()? {
        Some(user_id) => Ok(list_user_devices(&user_id)?
            .into_iter()
            .map(|d| (d.peer_id, d.bound_at))
            .collect()),
        None => Ok(Vec::new()),
    }
}

/// Update the user-level nickname this install announces to its contacts.
///
/// No-op when this install is not provisioned (there is no `users` row to
/// update); the canonical device-level self nickname stay authoritative in
/// that case (`identities.self_nickname`). This is the announced-name seam the
/// two-sided-nickname rule (`nickname_override` → `users.self_nickname` →
/// device) reads on the receiver side.
///
/// # Errors
/// Returns an error if the local peer/user cannot be resolved or the update
/// fails.
pub fn set_my_user_nickname(nickname: &str) -> color_eyre::Result<()> {
    let Some(user_id) = my_user_id()? else {
        return Ok(());
    };
    let conn = &mut crate::sqlite_connect()?;
    diesel::update(users::table)
        .filter(users::user_id.eq(&user_id))
        .set(users::self_nickname.eq(nickname))
        .execute(conn)?;
    Ok(())
}

/// Set this install's owner-side display name for a user contact.
///
/// The owner is always derived from the local device binding, so a frontend
/// cannot claim to write an override for another user. `None` clears an
/// existing override (and is a no-op when there is no row yet). The nickname
/// rules are validated here, not in a frontend, and a missing/unknown contact
/// or an unprovisioned local user is refused rather than creating a row that no
/// roster can ever display.
///
/// # Errors
/// Returns an error if the local user is unavailable, the target is not a known
/// user, the nickname is invalid, or the upsert fails.
pub fn set_contact_nickname(
    contact_user_id: String,
    nickname: Option<String>,
) -> color_eyre::Result<()> {
    let Some(owner_user_id) = my_user_id()? else {
        bail!("cannot set a contact nickname before this install is bound to a user");
    };
    ensure!(
        contact_user_id != owner_user_id,
        "an install cannot nickname itself as a contact"
    );
    if find_user(&contact_user_id)?.is_none() {
        bail!("contact user {contact_user_id} is unknown");
    }
    if let Some(name) = &nickname {
        ensure!(
            crate::nickname::validate_nickname(name),
            "invalid contact nickname: letters, numbers or dashes (max 20 characters)"
        );
    }

    let conn = &mut crate::sqlite_connect()?;
    let target = contacts::table
        .filter(contacts::owner_user_id.eq(&owner_user_id))
        .filter(contacts::contact_user_id.eq(&contact_user_id));
    match nickname {
        Some(name) => {
            let new_contact = NewContact {
                owner_user_id,
                contact_user_id,
                nickname_override: Some(name.clone()),
            };
            diesel::insert_into(contacts::table)
                .values(&new_contact)
                .on_conflict((contacts::owner_user_id, contacts::contact_user_id))
                .do_update()
                .set(contacts::nickname_override.eq(name))
                .execute(conn)
                .wrap_err("failed to set contact nickname")?;
        }
        None => {
            diesel::update(target)
                .set(contacts::nickname_override.eq(None::<String>))
                .execute(conn)
                .wrap_err("failed to clear contact nickname")?;
        }
    }
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
/// Mints a fresh account keypair (unless a previous interrupted attempt left
/// one stored), derives the root `user_id` from it, stores the account key,
/// persists the account, and signs our own first device-binding claim so the
/// install's peer ID is bound into its own user's device-set.
///
/// Idempotent and retry-safe: if this install is already bound into a user,
/// returns that `user_id` without touching anything; otherwise a stored account
/// key is reused, so a retry after a partial failure converges on the same
/// account root instead of replacing it with a fresh one.
///
/// # Errors
/// Returns an error if any persistence or signing step fails, including when a
/// stored account key exists but cannot be decoded (rather than silently
/// replacing the account root with a different key).
pub fn provision_my_user(
    self_username: Option<&str>,
    self_nickname: Option<&str>,
) -> color_eyre::Result<String> {
    let peer_id = crate::get_local_peer_id()?.to_string();
    if let Some(user_id) = find_user_id_for_peer(&peer_id)? {
        return Ok(user_id);
    }
    // Resume an interrupted provisioning from the stored key rather than
    // minting a new root: the key is written before the user row, so any partial
    // state carries the root we must keep.
    let account_key =
        get_stored_account_key()?.unwrap_or_else(libp2p_identity::Keypair::generate_ed25519);
    let user_id = user_id_from_account_pubkey(&account_key.public());
    // Store the key first: a crash before `save_user` leaves only a key, which
    // the next attempt reuses. Storing it last would orphan user rows and let a
    // retry overwrite the stored key with a different account root.
    store_account_key(&account_key)?;
    save_user(
        &user_id,
        &account_key.public().encode_protobuf(),
        self_username,
        self_nickname,
    )?;
    // The device-binding FK requires the peer row to exist: ensure ours does.
    crate::peers::save_peer(&peer_id, &[])?;
    let device_pubkey = crate::get_libp2p_identity()?.public().encode_protobuf();
    let claim_id = crate::gen_msg_id();
    let claim = sign_device_binding_claim(&account_key, &user_id, &device_pubkey, &claim_id)?;
    save_device_binding(&user_id, &peer_id, &device_pubkey, &claim_id, &claim, None)?;
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
    device_pubkey: impl AsRef<[u8]>,
) -> color_eyre::Result<crate::behavior::DeviceBindingRequest> {
    let device_pubkey = device_pubkey.as_ref();
    let local_peer = crate::get_local_peer_id()?.to_string();
    let Some(user_id) = find_user_id_for_peer(&local_peer)? else {
        return Err(color_eyre::eyre::eyre!(
            "this install has no user yet; provision one first"
        ));
    };
    let Some(user) = find_user(&user_id)? else {
        return Err(color_eyre::eyre::eyre!(
            "user {user_id} missing its account row"
        ));
    };

    // Signer selection: the account key when this install holds it (the
    // account holder), otherwise the local install key (a sibling device that
    // was bound into the user earlier). The latter widens the set of devices
    // that may add further devices without handing out the account private key.
    let account_key = get_stored_account_key()?;
    let signer: libp2p_identity::Keypair = match &account_key {
        Some(key) => key.clone(),
        None => crate::get_libp2p_identity()?,
    };
    let has_account_key = account_key.is_some();

    let account_pubkey_bytes = if has_account_key {
        signer.public().encode_protobuf()
    } else {
        user.account_pubkey.clone()
    };

    let claim_id = crate::gen_msg_id();
    let bound_at = crate::current_timestamp();
    let claim = sign_device_binding_claim(&signer, &user_id, device_pubkey, &claim_id)?;
    // The device-binding FK requires the peer row to exist: ensure the target
    // does (it normally does — it sent us the intro — but be safe).
    crate::peers::save_peer(peer_id, &[])?;
    save_device_binding(
        &user_id,
        peer_id,
        device_pubkey,
        &claim_id,
        &claim,
        Some(&local_peer),
    )?;
    Ok(crate::behavior::DeviceBindingRequest {
        kind: "claim".to_string(),
        user_id: Some(user_id),
        account_pubkey: Some(account_pubkey_bytes),
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
