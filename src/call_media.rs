//! Voice media transport for the calls lane (US-044).
//!
//! Signaling only establishes *that* two peers are talking; this module carries
//! the audio itself. It deliberately stays transport-only: capturing and playing
//! samples is a frontend adapter concern, so both the TUI and Flutter hand raw
//! 16 kHz mono PCM to the same Rust session and receive decoded PCM back.
//!
//! Wire format: one [`AudioFrame`] per 20 ms of audio (320 samples), G.711
//! μ-law encoded (one byte per sample), sent as its own request over
//! `/p2p-chat/call-media/1.0.0`. Each frame carries the caller-minted `call_id`
//! and a monotonically increasing sequence number, so stale, duplicated or
//! foreign frames can be dropped before they reach a speaker.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Number of PCM samples in one 20 ms frame at 16 kHz.
pub const SAMPLES_PER_FRAME: usize = 320;

/// Maximum number of buffered frames before the oldest is discarded.
const MAX_BUFFERED_FRAMES: usize = 24;

/// One μ-law encoded voice frame.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioFrame {
    /// Session this frame belongs to; echoed by every frame and the ack.
    pub call_id: String,
    /// Monotonic per-sender frame number, starting at zero.
    pub seq: u64,
    /// μ-law bytes, normally [`SAMPLES_PER_FRAME`] of them.
    pub samples: Vec<u8>,
}

/// Ack for one voice frame. Frames are fire-and-forget from the UI's view; the
/// ack only lets the sender distinguish a delivered frame from a dead peer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameAck {
    /// Whether the frame was accepted by the receiver.
    pub ok: bool,
    /// Human-readable reason when `ok` is false.
    pub reason: Option<String>,
}

/// CBOR codec for the voice-frame protocol (binary payloads, unlike signaling).
pub type MediaCodec = libp2p::request_response::cbor::codec::Codec<AudioFrame, FrameAck>;

/// G.711 μ-law bias, applied before the exponent/mantissa split.
const BIAS: i32 = 0x84;
/// Largest positive 16-bit linear sample before scaling.
const CLIP: i32 = 32_635;

/// Encode 16-bit PCM samples into μ-law bytes.
#[must_use]
pub fn pcm_to_mulaw(pcm: &[i16]) -> Vec<u8> {
    pcm.iter().copied().map(encode_sample).collect()
}

/// Decode μ-law bytes into 16-bit PCM samples.
#[must_use]
pub fn mulaw_to_pcm(samples: &[u8]) -> Vec<i16> {
    samples.iter().copied().map(decode_sample).collect()
}

/// Compand one sample (ITU-T G.711 μ-law, complemented sign-magnitude byte).
///
/// The magnitude is biased, then split into a segment exponent and a four-bit
/// mantissa. Positive samples XOR with `0xFF` (hence silence = `0xFF`),
/// negative ones with `0x7F`.
fn encode_sample(sample: i16) -> u8 {
    /// Exclusive upper bounds for the eight bias+segment ranges.
    const SEGMENT_ENDS: [i32; 8] = [0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000];

    let magnitude = i32::from(sample).saturating_abs().min(CLIP);
    let biased = magnitude.saturating_add(BIAS);
    let mask = if sample < 0 { 0x7F_i32 } else { 0xFF_i32 };
    let segment = SEGMENT_ENDS
        .iter()
        .position(|end| biased < *end)
        .unwrap_or(7);
    let quantization = (biased >> segment.saturating_add(3)) & 0x0F;
    let byte = i32::try_from(segment).unwrap_or(0).saturating_mul(16) | quantization;
    u8::try_from(byte ^ mask).unwrap_or(0xFF)
}

/// Expand one μ-law byte back to a 16-bit sample.
fn decode_sample(byte: u8) -> i16 {
    let value = u32::from(!byte);
    let negative = value & 0x80 != 0;
    let exponent = (value >> 4) & 0x07;
    let mantissa = value & 0x0F;
    let bias = u32::try_from(BIAS).unwrap_or(0);
    let magnitude = (mantissa << 3)
        .saturating_add(bias)
        .checked_shl(exponent)
        .unwrap_or(u32::MAX);
    let linear = i32::try_from(magnitude)
        .unwrap_or(i32::MAX)
        .saturating_sub(BIAS);
    let sample = i16::try_from(linear).unwrap_or(if negative { i16::MIN } else { i16::MAX });
    if negative {
        sample.saturating_neg()
    } else {
        sample
    }
}

/// Reordering and duplicate suppression for one inbound voice stream.
///
/// A single frame is held back so a packet that overtakes its predecessor can
/// still be played in order. Bounded storage means a hostile or broken peer
/// cannot grow the buffer without limit.
#[derive(Debug, Clone, Default)]
struct JitterBuffer {
    frames: BTreeMap<u64, AudioFrame>,
    last_played: Option<u64>,
}

impl JitterBuffer {
    /// Insert a frame, returning false for a duplicate/stale sequence number.
    fn push(&mut self, frame: AudioFrame) -> bool {
        if self.last_played.is_some_and(|last| frame.seq <= last)
            || self.frames.contains_key(&frame.seq)
        {
            return false;
        }
        if self.frames.len() >= MAX_BUFFERED_FRAMES {
            self.frames.pop_first();
        }
        self.frames.insert(frame.seq, frame);
        true
    }

    /// Take the next frame in sequence once at least two are buffered.
    fn take_ready(&mut self) -> Option<AudioFrame> {
        if self.frames.len() < 2 {
            return None;
        }
        let (&seq, _) = self.frames.iter().next()?;
        let frame = self.frames.remove(&seq)?;
        self.last_played = Some(seq);
        Some(frame)
    }
}

/// Media state for one live call.
///
/// The call id and peer id are pinned at creation: frames from another session
/// or another sender are rejected, so a late packet cannot bleed into a new
/// call that reuses the same audio adapter.
#[derive(Debug, Clone)]
pub struct MediaSession {
    /// Session id minted by the caller; echoed on every frame.
    pub call_id: String,
    /// Peer this session's audio is exchanged with.
    pub peer_id: String,
    next_seq: u64,
    muted: bool,
    inbound: JitterBuffer,
}

impl MediaSession {
    /// Open a media session for one already-accepted call.
    #[must_use]
    pub fn new(call_id: String, peer_id: String) -> Self {
        Self {
            call_id,
            peer_id,
            next_seq: 0,
            muted: false,
            inbound: JitterBuffer::default(),
        }
    }

    /// Whether a signal or frame with these ids belongs to this live session.
    ///
    /// Both the caller-minted `call_id` and the pinned peer must match: a
    /// foreign peer or a stale call id must never tear down or inject into a
    /// different call's audio.
    #[must_use]
    pub fn is_for(&self, call_id: &str, peer_id: &str) -> bool {
        self.call_id == call_id && self.peer_id == peer_id
    }

    /// Mute or unmute outgoing audio (inbound audio is unaffected).
    pub const fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// Encode one PCM block into the next outbound frame.
    ///
    /// Returns `None` when muted or when there is no audio to send, so a muted
    /// mic does not keep a 50 Hz frame stream alive for silence.
    #[must_use]
    pub fn next_frame(&mut self, pcm: &[i16]) -> Option<AudioFrame> {
        if self.muted || pcm.is_empty() {
            return None;
        }
        let seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        Some(AudioFrame {
            call_id: self.call_id.clone(),
            seq,
            samples: pcm_to_mulaw(pcm),
        })
    }

    /// Accept an inbound frame belonging to this session.
    ///
    /// Returns false for a foreign call id, a stale sequence number or a
    /// duplicate — all three are silently ignored by the caller.
    pub fn accept(&mut self, frame: AudioFrame) -> bool {
        frame.call_id == self.call_id && self.inbound.push(frame)
    }

    /// Take the next decodable inbound PCM block, if the jitter buffer has one.
    #[must_use]
    pub fn take_pcm(&mut self) -> Option<Vec<i16>> {
        let frame = self.inbound.take_ready()?;
        Some(mulaw_to_pcm(&frame.samples))
    }
}

#[cfg(test)]
#[path = "../tests/unit/unit_call_media.rs"]
mod tests;
