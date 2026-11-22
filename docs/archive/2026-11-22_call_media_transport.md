# Call media transport (2026-11-22)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/architecture.md](../architecture.md),
> [docs/README.md](../README.md).

This closes the transport half of **US-044** (voice/video calls). Call
*signaling* had shipped earlier (un-gated, mirroring device binding); this slice
adds the audio itself on a dedicated libp2p lane. It deliberately stops at the
transport boundary: capturing and playing samples is a frontend/device adapter
concern, so both frontends hand raw 16 kHz mono PCM to the same Rust session and
receive decoded PCM back. No device-level audio capture/playback adapter ships
yet — the Flutter side exposes a `CallMediaOverrides.audio` seam and the TUI
shows a frame count, which is the visible proof that media is flowing.

## Canonical model — `src/call_media.rs`

Rust owns the entire media state machine; the frontends only send commands and
render events. Key types:

- **`AudioFrame` / `FrameAck`** — the wire types. One frame carries the
  caller-minted `call_id`, a monotonic `seq`, and G.711 μ-law bytes (one per
  sample). Every frame is acked so a sender can tell a live receiver from a dead
  peer; the ack also reports rejection (`ok: false` + `reason`).
- **`MediaCodec`** — a CBOR `request_response` codec (binary payloads, unlike
  the signaling lane), registered as `/p2p-chat/call-media/1.0.0`.
- **`MediaSession`** — one live call, with the `call_id` and peer id pinned at
  creation. `next_frame(pcm)` μ-law-encodes one 20 ms / 320-sample block and
  numbers it; `accept(frame)` rejects foreign or stale frames; `set_muted`
  suppresses outgoing frames without touching inbound audio.
- **`JitterBuffer`** — a bounded `BTreeMap<u64, AudioFrame>` holding one frame
  back so a packet that overtakes its predecessor is still played in order.
  Duplicates and stale sequence numbers are dropped; the buffer is capped at
  24 frames so a broken peer cannot grow it without limit.
- `pcm_to_mulaw` / `mulaw_to_pcm` are the reference compander/expander. Tests
  pin the ITU-T reference vectors (`encode(0)=0xFF`, `encode(-1)=0x7F`) and a
  ±1-quantization-step roundtrip bound.

No new Cargo media dependencies: μ-law is a small in-tree codec and the `calls`
Cargo feature stays empty/reserved (signaling is not gated behind it).

## Swarm wiring — `src/behavior.rs`, `src/swarm_handler.rs`

`AppBehaviour.call_media` is a new request-response behaviour built alongside
the signaling lane. The swarm loop owns a single `Option<MediaSession>` (one
call at a time, like the call session slot). New commands:
`StartCallMedia { call_id, peer_id }`, `SendCallMedia { call_id, pcm }`,
`SetCallMediaMute { muted }`, `StopCallMedia`. Inbound frames produce
`SwarmEvent::CallMediaFrame { call_id, peer_id, pcm }` with decoded PCM.

Guards mirror the other lanes: the media lane is blocklist-gated at the swarm
boundary; frames are accepted only from the exact peer and call id of the open
session; `reject`/`end` call signals clear the session so a late frame from an
old call cannot bleed into the next one; a stale `call_id` on `SendCallMedia` is
ignored rather than leaking audio. Nothing is persisted and nothing is logged
per frame (50 Hz would drown the log tab).

## Frontend parity

- **FRB/web surface**: `api::{start_call_media, stop_call_media,
  send_call_media, set_call_media_mute}` (FRB-exposed wrappers over
  `#[frb(ignore)]` `mobile_node` impls that queue the commands), mirrored by
  `startCallMedia` / `stopCallMedia` / `sendCallMedia` / `setCallMediaMute` JSON
  arms in `src/web/methods.rs` with drift guards in `src/web/mod.rs`.
  `SwarmEventJson.audio: Option<Vec<i16>>` carries decoded PCM on the
  `call_media` event (`content` = call id, `peer_id` = speaker). FRB bindings
  were regenerated with 2.13.0.
- **Flutter**: `NodeApi` gains the four commands; `FrbNodeApi` and
  `HttpNodeApi` implement them over the in-process and WS/HTTP transports.
  `CallScreen` opens the media session on accept / `call_accepted`, closes it on
  hang-up, reject, end, tab-leave and dispose, routes `call_media` events to a
  `CallMediaOverrides.audio` seam, offers a `sendPcm` hook for a future
  microphone adapter, adds a mute toggle, and shows `Mic on/muted · peer frames
  N`. The event sink filters on peer id + echoed call id (unchanged).
- **TUI**: `AppState.call_media: CallMediaStatus` tracks mute state and a
  decoded-frame count. `a` (accept) starts media, `m`/`M` toggles mute, and
  cancel/hang-up sends `StopCallMedia`. The in-call overlay renders the same
  mic/frame status. The terminal owns no audio device, so this is status only.

## Tests

- `tests/unit/unit_call_media.rs`: reference μ-law vectors, roundtrip bound,
  jitter reordering/bounded/dedup, foreign/stale frame rejection, mute,
  empty-input helpers.
- `tests/swarm_handler.rs::test_spawn_handler_call_media_roundtrip`: two swarms
  exchange CBOR frames on the live lane; the receiver's jitter buffer releases
  decoded PCM with the authenticated peer/call ids.
- `apps/flutter_app/test/widget/call_screen_test.dart`: media start/stop on
  accept/hang-up, mute toggle + status line, inbound PCM routing with foreign
  session filtering, and `sendPcm` gating while in call. These drive the
  `CallMediaOverrides` and `FakeNodeApi` seams, so no FFI is loaded.
- TUI call-overlay tests now assert the new `StartCallMedia` / `StopCallMedia`
  commands (and use channel capacity > 1, since the swarm loop drains
  continuously in production but those tests drive the processor directly).
