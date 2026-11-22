#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction,
    clippy::used_underscore_binding,
    clippy::significant_drop_tightening
)]

use super::*;

#[test]
fn mulaw_reference_vectors() {
    assert_eq!(encode_sample(0), 0xFF);
    assert_eq!(encode_sample(-1), 0x7F);
    assert_eq!(encode_sample(i16::MAX), 0x80);
    assert_eq!(encode_sample(i16::MIN), 0x00);
    assert_eq!(decode_sample(0xFF), 0);
    assert_eq!(decode_sample(0x7F), 0);
}

#[test]
fn mulaw_roundtrip_is_within_one_quantization_step() {
    let pcm: Vec<i16> = vec![
        0, 1, -1, 100, -100, 1_000, -1_000, 8_000, -8_000, 32_000, -32_000,
    ];
    let decoded = mulaw_to_pcm(&pcm_to_mulaw(&pcm));
    assert_eq!(decoded.len(), pcm.len());
    for (original, restored) in pcm.iter().zip(&decoded) {
        assert!(
            (i32::from(*original) - i32::from(*restored)).abs() <= 1024,
            "{original} -> {restored}"
        );
    }
}

#[test]
fn pcm_helpers_handle_empty_input() {
    assert!(pcm_to_mulaw(&[]).is_empty());
    assert!(mulaw_to_pcm(&[]).is_empty());
}

#[test]
fn jitter_buffer_plays_in_sequence_and_drops_duplicates() {
    let mut buffer = JitterBuffer::default();
    let frame = |seq| AudioFrame {
        call_id: "c1".into(),
        seq,
        samples: vec![seq as u8],
    };
    assert!(buffer.push(frame(1)));
    assert!(!buffer.push(frame(1)));
    // One frame is held back for reordering.
    assert!(buffer.take_ready().is_none());
    assert!(buffer.push(frame(2)));
    assert_eq!(buffer.take_ready().map(|f| f.seq), Some(1));
    assert!(!buffer.push(frame(1)));
    assert!(buffer.take_ready().is_none());
    assert!(buffer.push(frame(3)));
    assert_eq!(buffer.take_ready().map(|f| f.seq), Some(2));
}

#[test]
fn jitter_buffer_is_bounded() {
    let mut buffer = JitterBuffer::default();
    for seq in 0..(MAX_BUFFERED_FRAMES + 5) {
        assert!(buffer.push(AudioFrame {
            call_id: "c1".into(),
            seq: seq as u64,
            samples: Vec::new(),
        }));
    }
    assert_eq!(buffer.frames.len(), MAX_BUFFERED_FRAMES);
    assert_eq!(
        buffer.frames.first_key_value().map(|(seq, _)| *seq),
        Some(5)
    );
}

#[test]
fn session_numbers_frames_and_decodes_inbound_audio() {
    let mut session = MediaSession::new("c1".into(), "peer-1".into());
    let first = session.next_frame(&[0; SAMPLES_PER_FRAME]).expect("frame");
    assert_eq!(first.seq, 0);
    assert_eq!(first.call_id, "c1");
    assert_eq!(first.samples.len(), SAMPLES_PER_FRAME);
    assert_eq!(
        session
            .next_frame(&[0; SAMPLES_PER_FRAME])
            .expect("frame")
            .seq,
        1
    );

    // Inbound frames arrive out of order; playback waits for the gap to fill.
    assert!(session.accept(AudioFrame {
        call_id: "c1".into(),
        seq: 2,
        samples: pcm_to_mulaw(&[100; SAMPLES_PER_FRAME]),
    }));
    assert!(session.take_pcm().is_none());
    assert!(session.accept(AudioFrame {
        call_id: "c1".into(),
        seq: 1,
        samples: pcm_to_mulaw(&[100; SAMPLES_PER_FRAME]),
    }));
    let pcm = session.take_pcm().expect("decoded frame");
    assert_eq!(pcm.len(), SAMPLES_PER_FRAME);
    assert!((i32::from(pcm[0]) - 100).abs() <= 1024);
}

#[test]
fn session_rejects_foreign_and_stale_frames() {
    let mut session = MediaSession::new("c1".into(), "peer-1".into());
    let frame = |call_id: &str, seq| AudioFrame {
        call_id: call_id.into(),
        seq,
        samples: vec![0],
    };
    assert!(!session.accept(frame("other-call", 0)));
    assert!(session.accept(frame("c1", 0)));
    assert!(!session.accept(frame("c1", 0)));
    assert!(session.accept(frame("c1", 1)));
    assert_eq!(session.take_pcm().expect("frame").len(), 1);
    assert!(!session.accept(frame("c1", 0)));
}

#[test]
fn muted_session_sends_nothing_but_still_receives() {
    let mut session = MediaSession::new("c1".into(), "peer-1".into());
    session.set_muted(true);
    assert!(session.is_muted());
    assert!(session.next_frame(&[1; SAMPLES_PER_FRAME]).is_none());
    session.set_muted(false);
    assert!(session.next_frame(&[]).is_none());
    assert!(session.next_frame(&[1; SAMPLES_PER_FRAME]).is_some());
}
