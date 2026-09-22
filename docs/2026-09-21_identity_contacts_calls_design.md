# 2026-09-21 — Identity, device-binding, contacts, and (feature-gated) calls: architectural design

Lane: **backlog §11** (user/device identity + contact list first + calls later, feature-gated).
This doc is the architecture that lane. It is byte-anchored to the real surfaces so the
implementation lane is mechanical.

## Scope & status

**Design, not implementation.** Records the two-layer identity graph, the signed-claim root of
trust, the contacts-first sequencing, and the calls feature-gate. Implementation is a separate
lane (after this lands).

## Current state (byte-verified)

- **One keypair per install**: `src/db.rs` load-or-generates a single libp2p identity keypair;
  a fresh install → a fresh keypair. Stored in the `identities` table (`key` → Binary, Ed25519,
  with `last_tcp_port`/`last_quic_port`/`self_nickname`).
- **No user concept exists**: every message/recipient/session/group row keys on `peer_id`
  (the device). A "user" today is implicitly one device; multi-device-per-user, contacts, and
  calls have **no schema, no binding, no FRB surface** — confirmed by repo-wide grep.
- `peer_id` **is** identity: DMs address a peer_id, groups are peer-membered. Nothing changes in
  how the mesh transports/keys work; the identity layer sits *above* the transport.

## The two-layer identity graph

### Layer 1 — account identity (the user, root of trust)

A **user** is a stable account pubkey. `user_id` is a self-verifying, centrally-registry-free
fingerprint (multihash of the account pubkey), so a user can be referred to by key alone with no
lookup service.

- The account key is the **only** signer that can bind devices into that user's device-set.
- A per-install keypair holds only its own libp2p key — **not** the account key — so any device
  can mint claims **only inside the user identity it was provisioned under**, and can never mint
  a binding for a **foreign** user's set. This is the §11.1 hostile-device invariant, enforced
  cryptographically (absent the foreign account key, a forged claim fails verification at any
  receiver).

### Layer 2 — device binding (the device)

Each install keeps its existing per-install libp2p keypair and `peer_id`. What's added is a
*signed binding* of that peer_id into exactly one user's device-set:

```
claim = sign_{account}(user_id ‖ device_pubkey ‖ claim_id ‖ bound_at)
```

- Ruletransforms: a device row is **only** trusted when its claim verifies against the claimed
  user's account pubkey. Any peer verifies before trusting the binding.
- One row per `(user_id, peer_id)`; revocation flips `revoked_at` (the account key re-signs).

### New crate surface

| Surface | Purpose |
|---|---|
| `users` — `user_id`, `account_pubkey`, `created_at`, `self_nickname` | the account/identity rows |
| `device_bindings` — `claim_id`, `user_id`, `peer_id`, `device_pubkey`, `claim` (sig bytes), `bound_at`, `revoked_at` (null = live) | the signed device-set |
| `contacts` — `owner_user_id`, `contact_user_id`, `nickname_override`, `added_at`, `last_seen` | the contact list (users, not peers) |
| `identities` (existing) | unchanged — still the per-install keypair |

Messages/receipts/sessions/groups stay peer/group-keyed; only the *app-layer addressing* grows a
user→device resolution. No DB migration touches the message lanes.

## FRB / bindings (one committed block)

New canonical fn per concept (following the item-9/10 single-binding rule — one Rust fn, one
binding, Dart calls it, no Dart reimplementation, no `hide` clauses):

- `api::get_my_user_id` — this install's user (resolves `identities` + its binding).
- `api::add_device(user_id, claim)` — verify claim against user's account pubkey, then store.
- `api::get_user_devices(user_id)` — bounded (recent-N) device-set.
- `api::verify_device_binding_claim(claim)` — the pure verify (also serves `add_device`).
- `api::get_contacts` / `api::add_contact(user_id, nickname_override)` / `api::remove_contact`.
- `validate_nickname` / `network_size_label` stay as the existing single bindings (not re-touched).

Requires the FRB regeneration protocol from AGENTS.md (codegen `=2.13.0` → re-append clippy
allows → `cargo fmt --all` → delete untracked `.h` → `cargo ct` + `cargo test --all-features` +
`flutter analyze`/`flutter test`). The whole bindings change lands as **one** commit.

## Transport lanes

- **Claims** travel the existing request/response DM lane — receiver stores only after
  signature-verification succeeds.
- **Backfill** (store-and-forward for offline members) stays **deferred** — §11.3/11.4 recorded
  the sub-decisions (gossipsub vs request_response; watermark persisted vs on-demand) as open.
- **Calls/videocalls** are **feature-gated**: a `calls = ["dep:..."]` Cargo feature, **off by
  default, excluded from `mobile`**. When the calls lane lands (after contacts), all media deps
  (WebRTC/audio codecs) live behind that feature so default/mobile builds stay lean. This doc
  records the gate; the calls lane is where the feature + deps actually land.

## Sequencing — contacts first, calls later

1. **Identity layer**: `users` + `device_bindings` + signed-claim verification (the root-of-trust
   prerequisite). New bindings listed above (minus contacts).
2. **Contacts**: `contacts` table + `get_contacts`/`add_contact`/`remove_contact` + Dart surface.
3. **Calls/videocalls** (later, feature-gated): break the `calls` gate, add media deps, gate kept
   so unneeded builds skip them. Uses the call vs. user identity; multi-device lanes follow §11.

## Explicit deferrals (recorded, not forgotten)

- Backfill transport choice + watermark persistence location — §11.3, implementation lane.
- Who may *add* a sibling device (device-signed vs. account-key-only) — §11.5, implementation lane.
- Same-user message sync across devices requires ≥1 of the user's devices online within the gap —
  no external store-and-forward (the §11 note; not a separate promised server lane).

## Verification (this doc)

`git status` clean; one commit `docs(design): ...`.

Commits: this commit (design doc) — implementation lane is a separate later lane.
