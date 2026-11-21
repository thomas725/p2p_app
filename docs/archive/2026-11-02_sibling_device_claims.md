# Sibling-device claim authority (2026-11-02)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/identity.md](../identity.md).

This closes the P0 decision recorded in the original §11 "open sub-decisions":
**who may add a device to a user's set**. The decision: any already-bound device
of the same user may add another, alongside the account key. The account key
keeps working; this only widens the set of authorized signers.

## Decision

A device-binding claim (`user_id ‖ device_pubkey ‖ claim_id`) is accepted when it
verifies against either:

1. the claimed user's `users.account_pubkey` (the original account-key path), or
2. the **sending** device's stored `device_pubkey`, gated on that sender already
   being bound to the claimed `user_id` (the new sibling path).

Trying only the sender's key — rather than every bound key of the user — is
deliberately the narrowest change that achieves the goal. The receiver learns
the sender from the request's peer id, so the wire claim needs no signer field.

`mint_device_binding` falls back to signing with the local install key when no
account key is stored, so a bound sibling device can mint claims for new devices.

## Hostile-device invariant

Unchanged: a device can never bind into a **foreign** user's set. A claim is only
accepted if it verifies against the claimed user's account key or against the
device key of a sender already bound to that same `user_id`. An unbound signer
or a device bound to a different user is rejected.

The accepted residual risk is deliberate: a compromised bound device can add
further devices (persistence). It already holds the user's message history, so
the usability gain outweighs it. Unbinding a device does not retroactively
invalidate claims it signed.

## Tests

`tests/unit/unit_identity.rs` covers sibling mint + verify, rejection of an
unbound signer, rejection of a signer bound to a different user, and the
account-key path still working. Implementation: commit `ede126b`.
