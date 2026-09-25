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

## Contacts tab (replaces the Peers tab) — user decision (2026-09-24)

Recorded from the user directly (the "Dart surface" of step 2). Applies to **both** frontends
(TUI + Flutter) to keep the shared parity rules (single canonical model, one Rust fn per binding,
TUI/Flutter sort parity).

- **Replace the Peers tab with a Contacts tab.** The tab's rows become *contacts*: unbound peers
  still appear (as single-device rows, exactly today's behavior), and peers that resolve to a user
  via `device_bindings` are grouped under that user.
- **Single-device contact ≈ today's peer row.** Minimal behavior change; click/selection/message
  flows stay as they are (peer click → DM, message click → sender's PeerInfo).
- **Multi-device contact = one row.** A user with several bound devices shows a single contact row
  aggregating all of them (device count, and the device rows are reachable/expandable from the
  contact — exact expand behavior lands with the TUI/Flutter implementation).
- **Two-sided nicknames.** Devices *and* users may each carry a nickname from the owner's side
  (`peer_local_nickname` for a device, `contacts.nickname_override` for a user) and from the
  receiving user's side (`received_nickname`, `users.self_nickname`); the two may differ. The
  contact row's display name resolves: user `nickname_override` → user `self_nickname` → best
  device display name, keeping the existing `get_peer_display_name` precedence (local → received
  → petname) for the device level.

### Canonical backend shape (both frontends render against this)

A single Rust model + one bulk loading fn feeds both UIs (replacing the peer-list path):

- `ContactRow` = `contact_id` (user_id or peer_id), display name, `is_user`, device list,
  aggregated stats (dm_count, broadcast_sent_to_peer summed across the user's devices), aggregate
  `first_seen`/`last_seen` (min/max across the group), and the per-device rows.
- Grouping: reverse `device_bindings` lookup (`find_user_id_for_peer`) over `load_peers`; build a
  `HashMap<Option<user_id>, Vec<peer>>`. Bulk `get_all_peer_stats` provides counts (same two-pass
  query) which are summed per group.
- Sort parity: the TUI sort and the Flutter `sortPeers` binding keep the deterministic rules, now
  over contact rows (ties on the contact's stable key = the user_id, or peer_id for unbound rows).
- Display-name rule above lives in `nickname.rs` so both frontends agree; the existing
  `DISPLAY_NAME_CACHE` invalidation points stay the source of freshness.

## Explicit deferrals (recorded, not forgotten)

- Backfill transport choice + watermark persistence location — §11.3, implementation lane.
- Who may *add* a sibling device (device-signed vs. account-key-only) — §11.5, implementation lane.
- Same-user message sync across devices requires ≥1 of the user's devices online within the gap —
  no external store-and-forward (the §11 note; not a separate promised server lane).

## Provisioning + claims exchange (landed 2026-09-25) — resolved decisions

First implementation slice of the identity layer. Three decisions resolved (deviations from the
proposal above are bolded):

- **Claim message layout keeps `bound_at` out of the signed bytes.** Signed message =
  `user_id ‖ device_pubkey ‖ claim_id` (NOT `... ‖ claim_id ‖ bound_at` as drawn at line 45).
  Shared `identity::device_binding_claim_message` builds it byte-for-byte for both the verifier and
  the minters, so nothing can drift. `bound_at` is carried in the claim record for display/ordering
  only (it is mint-time, visible on the claim record itself).
- **`identities` gains a nullable `account_key BLOB`** (line 59 said "identities unchanged").
  Deviation: sibling-device claims can only be minted by the account holder; since the account key
  cannot be reconstructed from the account pubkey, the install that provisions must persist it. The
  swap of the per-install keypair to the account key stays **rejected** (that would collapse
  multi-device identity); account_key is a *retained* addition, seeded by
  `api::provision_my_user` (idempotent) — re-provisioning never changes the account root.
- **Mobile auto-provisions at node start** (`mobile_node::start_node_impl`): every install
  provisions a fresh user identity on first start (idempotent; later starts reuse the account).
  No "create account" UI is needed — the Contacts tab grouping just always applies. The Flutter
  app surfaces incoming `device_binding_intro` events as an explicit **Allow/Decline** dialog
  (`promptDeviceBinding`, `@visibleForTesting` accept seam like `SortContactsOverride`) whose
  Allow calls `acceptDeviceBinding`; `device_binding_received` refreshes the roster. The
  initiating direction (device advertises itself to a peer's account) is the per-device
  **link** action in the Contacts table (`onLinkDevice` → `sendDeviceBindingIntro`) — the
  "add my second device" flow: run it on the new device, targeting the account holder already
  in the roster.
- **Receivers never auto-mint on an intro.** An incoming `intro` is surfaced as an event only
  (`SwarmEvent::DeviceBindingIntro`, later `DeviceBindingReceived { user_id }` after the account
  holder calls `accept_device_binding`); the holder explicitly accepts. Claims (`kind: "claim"`)
  are verified and bound by `swarm_handler::verify_and_bind_claim` only after the account-signed
  claim passes, enforcing the §11.1 hostile invariant.
- Transport: `device_binding` request/response behaviour (protocol id
  `/p2p-chat/device-binding/1.0.0`) minted/consumed by `identity::{intro,mint,verify}_device_binding*`
  and routed/managed in `swarm_handler`; sends are queued as `SwarmCommand::SendDeviceBinding` and
  issued through the existing `request_response` machinery (carries `Box<DeviceBindingRequest>`).
- **TUI parity for intros (landed with the mobile slice):** the TUI auto-provisions at startup
  (same idempotent `provision_my_user(None, None)` call as mobile) and surfaces
  `DeviceBindingIntro` as a **modal prompt** (single-slot `AppState.device_binding_prompt`,
  rendered as the window popup): `a` accepts — mints the signed claim via
  `identity::mint_device_binding` and queues `SwarmCommand::SendDeviceBinding` — and any other
  key (incl. `d`/Esc) declines. Explicit-accept only, mirroring the mobile Allow/Decline
  dialog; `DeviceBindingReceived` shows a confirmation popup. Tested in the bin input/render
  suites (bin 249).
- Verification gate: `cargo ct` clean, `cargo test --all-features` green (lib 299, bin 244,
  integration suites incl. new identity provisioning/mint/hostile-claim tests), `flutter analyze`
  clean, `flutter test` 18/18. Committed as its own slice.

## Verification (this doc)

`git status` clean; one commit `docs(design): ...`.

Commits: this commit (design doc) — implementation lane is a separate later lane.
