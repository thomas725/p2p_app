> **ARCHIVED** — historical record, not current behaviour.

# Device-centric location sharing (2026-11-27)

Location sharing is a user-visible slice made up of several small policies:
which devices may receive a fix, when a sensor or network send is due, how a
fix is rendered, and how one device's track is exported. Putting those rules in
two frontends would make the TUI, Flutter and browser disagree about the same
message, so the whole policy lives in Rust and the frontends only move raw
samples and render Rust-owned strings.

## What shipped

- `src/location.rs` validates coordinates and accuracy, renders the canonical
  `[Location] lat,lon (±accuracy m) @seconds URL (mode summary) [#sequence]
  [(final)]` body, and parses that body back into a flat `LocationTrackPoint`.
  Only a body with the canonical OSM URL is accepted; a hand-written prefix is
  ordinary chat text, not a track point.
- One-point URLs centre an OSM marker; tracks use OSM directions with the
  project's walking engine and are capped at 50 waypoints. GPX exports stay
  unbounded, escape the device label and carry accuracy in an extension.
- Sharing modes are `once`, `limited` and `unlimited`. Bounds, defaults,
  persisted settings, expiry, sensor cadence and send cadence are all checked
  by the same Rust functions; the frontends only pass wall-clock seconds.
- Recipients are *other devices bound to the local user*, resolved in Rust from
  device-binding rows. A DM partner is not implicitly eligible. Track loading
  keys on one exact sending `peer_id`, so two devices of one account never
  mix into a single GPX file.
- Location rides the existing sealed DM lane (`send_outgoing_dm`), so there is
  no new wire protocol, message table or column. At-rest protection remains a
  database-level concern, as documented in the backlog.
- The TUI offers `Ctrl+L` on a Direct tab: a modal `from;to` prompt is parsed by
  Rust, and the resulting GPX plus a plain-text OSM URL are shown in a notice.
  Inline terminal hyperlinks are deliberately deferred.
- Flutter uses the same `NodeApi` surface through either in-process FRB or the
  web JSON transport. Android samples real GPS/network fixes through a
  `p2p_app/location` method channel that also owns the runtime permission
  prompt; the browser uses `dart:js_interop` Geolocation and DeviceMotion with
  explicit secure-context and denial errors. Both expose a low-power motion
  gate that fails open when no sensor is present.
- Settings edits are persisted as validated JSON and read through one Rust
  helper. Bad or corrupt rows fall back to mode `once`, a 10-second sensor
  interval, a 60-second send interval and motion gating enabled.

## Design decisions

- The canonical body is deliberately plain text inside a sealed DM. It needs
  no new frontend message type, persists in the existing history query, and
  renders identically in every frontend even without a location-aware UI.
- Coordinates, time filters, GPX and URL assembly stay in Rust. Dart and
  Kotlin never compute a bounding window, format a coordinate or parse an
  ISO date; that prevents the browser and the TUI from drifting.
- FRB 2.13 cannot generate `Option<&str>`, so the track loader takes owned
  `String` bounds. The web JSON mirrors the same camelCase names.
- Eligibility is checked twice: `listLocationRecipients` only shows the
  devices the local account may target, and `sendLocationToDevices` re-resolves
  and validates every selected peer id in Rust before persisting anything.
- Browser permission is implicit in Geolocation, so the adapter reports a
  denial reason instead of a separate request call. Android needs an explicit
  runtime prompt; the native side queues deferred replies so `close()` and a
  denial can never leave a Dart future hanging.

## Tests

- `tests/unit/unit_location.rs` covers validation, canonical round-tripping,
  mode/expiry/cadence rules, relative and absolute time filters, GPX escaping
  and the capped track URL.
- `tests/unit/unit_settings.rs` covers persisted defaults and invalid rows.
- `tests/location_integration.rs` exercises real SQLite history, device-scoped
  filtering, time windows and recipient resolution with two sibling devices.
- `tests/tui_render_integration.rs` proves the canonical body renders as plain
  text on a DM tab; TUI export tests cover filter splitting, sanitised file
  names and GPX contents.
- Web dispatch tests assert every new FRB method has a matching JSON arm.
  Dart unit and widget tests cover both transports, permission/secure-context
  reporting and the sharing/export flows.

## Related records

- [`../architecture.md`](../architecture.md)
- [`../backlog.md`](../backlog.md)
- [`2026-11-18_us-042_e2e_dm.md`](2026-11-18_us-042_e2e_dm.md)
- [`2026-11-19_store_and_forward_dm_outbox.md`](2026-11-19_store_and_forward_dm_outbox.md)
