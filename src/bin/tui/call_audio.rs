//! Optional desktop microphone/speaker adapter for the TUI calls overlay.
//!
//! This module is deliberately split in two by the `tui-audio` Cargo feature:
//!
//! - with the feature, [`TuiAudioHandle`] owns a `cpal` capture/playback thread
//!   that hands 16 kHz mono `i16` frames to the same Rust media session the
//!   Flutter adapters use;
//! - without it, the handle is a no-op and the overlay keeps its frame counter.
//!
//! `tui-audio` is NOT part of `default` or `tui`. The headless server, the
//! Android library and normal TUI builds therefore compile **no** `cpal` code
//! and need no host audio libraries. Mute, framing and jitter still belong to
//! `p2p_app::call_media::MediaSession`; this adapter only moves raw PCM.

#[cfg(feature = "tui-audio")]
mod enabled {
    use cpal::Sample;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use num_traits::ToPrimitive;
    use p2p_app::SwarmCommand;
    use p2p_app::call_media::SAMPLES_PER_FRAME;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
    use std::thread::Builder;
    use std::time::Duration;
    use tokio::sync::mpsc;

    /// Wire sample rate shared with the browser, Android and Rust media code.
    const WIRE_RATE_HZ: u32 = 16_000;
    /// Keep well under a second of PCM in either direction; overload drops
    /// samples rather than delaying a live conversation.
    const MAX_CAPTURE_SAMPLES: usize = 16_000;
    const MAX_PLAYBACK_SAMPLES: usize = 16_000;
    /// One 20 ms frame at the wire rate.
    const FRAME_INTERVAL: Duration = Duration::from_millis(20);

    /// Commands from the TUI tasks to the dedicated audio thread.
    enum Control {
        Stop,
        Play(Vec<i16>),
    }

    /// State shared with the audio thread. `control` is taken on stop; `errors`
    /// are drained into `CallMediaStatus` by the render loop.
    #[derive(Debug, Default)]
    struct Shared {
        control: Option<std::sync::mpsc::Sender<Control>>,
        errors: Vec<String>,
    }

    /// Cloneable, feature-gated handle to the audio thread.
    #[derive(Clone, Debug, Default)]
    pub struct TuiAudioHandle {
        shared: Arc<Mutex<Shared>>,
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().unwrap_or_else(PoisonError::into_inner)
    }

    impl TuiAudioHandle {
        /// Start capture/playback for one accepted call. A second start while a
        /// session is live is ignored, matching the single-call UI contract.
        pub fn start(
            &self,
            call_id: &str,
            peer_id: &str,
            swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
        ) {
            let swarm_cmd_tx = swarm_cmd_tx.clone();
            let (control_tx, control_rx) = std::sync::mpsc::channel();
            {
                let mut shared = lock(&self.shared);
                if shared.control.is_some() {
                    return;
                }
                shared.control = Some(control_tx);
            }

            let handle = self.clone();
            let call_id = call_id.to_owned();
            let _ = peer_id;
            let spawned = Builder::new()
                .name("p2p-tui-audio".to_owned())
                .spawn(move || {
                    if let Err(error) = run_audio(&call_id, &swarm_cmd_tx, &control_rx) {
                        handle.report_error(error);
                    }
                    lock(&handle.shared).control = None;
                });
            if let Err(error) = spawned {
                lock(&self.shared).control = None;
                self.report_error(format!("Could not start audio thread: {error}"));
            }
        }

        /// End capture/playback. The audio thread also clears its own slot when
        /// it exits, so a late stop is harmless.
        pub fn stop(&self) {
            let control = lock(&self.shared).control.take();
            if let Some(sender) = control {
                let _ = sender.send(Control::Stop);
            }
        }

        /// Queue decoded peer PCM for the speaker.
        pub fn play(&self, pcm: &[i16]) {
            let shared = lock(&self.shared);
            if let Some(sender) = shared.control.as_ref() {
                let _ = sender.send(Control::Play(pcm.to_vec()));
            }
        }

        /// Take the first device error reported by the audio thread, if any.
        #[must_use]
        pub fn take_error(&self) -> Option<String> {
            lock(&self.shared).errors.pop()
        }

        fn report_error(&self, message: String) {
            lock(&self.shared).errors.push(message);
        }
    }

    /// Linear resampler with a fractional read cursor into a short queue.
    ///
    /// `from_hz / to_hz` drives the cursor: capture uses the device rate to the
    /// wire rate, playback the wire rate to the device rate. Linear
    /// interpolation avoids needing a resampler dependency while keeping pitch
    /// correct when the sound card is 44.1/48 kHz.
    #[derive(Debug)]
    struct RateConverter {
        step: f64,
        cursor: f64,
        samples: VecDeque<f32>,
    }

    impl RateConverter {
        fn new(from_hz: u32, to_hz: u32) -> Self {
            Self {
                step: f64::from(from_hz) / f64::from(to_hz),
                cursor: 0.0,
                samples: VecDeque::new(),
            }
        }

        fn push(&mut self, input: &[f32], output: &mut Vec<f32>) {
            self.samples.extend(input.iter().copied());
            loop {
                let available = self.samples.len().to_f64().unwrap_or(f64::MAX);
                if available <= self.cursor + 1.0 {
                    break;
                }
                let index = self.cursor.to_usize().unwrap_or(0);
                let fraction = self.cursor - self.cursor.floor();
                let first = self.samples.get(index).copied().unwrap_or(0.0);
                let second = self
                    .samples
                    .get(index.saturating_add(1))
                    .copied()
                    .unwrap_or(first);
                output.push((second - first).mul_add(fraction.to_f32().unwrap_or(0.0), first));
                self.cursor += self.step;
            }
            let consumed = self.cursor.to_usize().unwrap_or(0);
            if consumed > 0 {
                drop(self.samples.drain(..consumed));
                self.cursor -= consumed.to_f64().unwrap_or(0.0);
            }
        }
    }

    /// Capture-side state: device-rate mono samples in, wire-rate `i16` frames
    /// out. Interleaved device channels are averaged to mono first.
    #[derive(Debug)]
    struct CaptureState {
        converter: RateConverter,
        pcm: VecDeque<i16>,
    }

    impl CaptureState {
        fn new(device_rate_hz: u32) -> Self {
            Self {
                converter: RateConverter::new(device_rate_hz, WIRE_RATE_HZ),
                pcm: VecDeque::new(),
            }
        }

        fn push<T>(&mut self, data: &[T], channels: usize)
        where
            T: cpal::SizedSample,
            f32: cpal::FromSample<T>,
        {
            let mono = to_mono(data, channels);
            let mut resampled = Vec::with_capacity(mono.len());
            self.converter.push(&mono, &mut resampled);
            for sample in resampled {
                if self.pcm.len() >= MAX_CAPTURE_SAMPLES {
                    break;
                }
                self.pcm
                    .push_back(sample.clamp(-1.0, 1.0).to_sample::<i16>());
            }
        }

        fn take_frame(&mut self) -> Option<Vec<i16>> {
            if self.pcm.len() < SAMPLES_PER_FRAME {
                return None;
            }
            Some(
                (0..SAMPLES_PER_FRAME)
                    .filter_map(|_| self.pcm.pop_front())
                    .collect(),
            )
        }
    }

    /// Playback-side state: wire-rate PCM in, device-rate mono samples out.
    #[derive(Debug)]
    struct PlaybackState {
        converter: RateConverter,
        samples: VecDeque<f32>,
    }

    impl PlaybackState {
        fn new(device_rate_hz: u32) -> Self {
            Self {
                converter: RateConverter::new(WIRE_RATE_HZ, device_rate_hz),
                samples: VecDeque::new(),
            }
        }

        fn enqueue(&mut self, pcm: &[i16]) {
            let wire: Vec<f32> = pcm
                .iter()
                .copied()
                .map(cpal::Sample::to_sample::<f32>)
                .collect();
            let mut resampled = Vec::with_capacity(wire.len());
            self.converter.push(&wire, &mut resampled);
            self.samples.extend(resampled);
            while self.samples.len() > MAX_PLAYBACK_SAMPLES {
                self.samples.pop_front();
            }
        }

        fn take_mono(&mut self, frames: usize) -> Vec<f32> {
            (0..frames)
                .map(|_| self.samples.pop_front().unwrap_or(0.0))
                .collect()
        }
    }

    /// Average interleaved device channels into mono normalized samples.
    fn to_mono<T>(data: &[T], channels: usize) -> Vec<f32>
    where
        T: cpal::SizedSample,
        f32: cpal::FromSample<T>,
    {
        let channels = channels.max(1);
        let divisor = channels.to_f32().unwrap_or(1.0);
        data.chunks(channels)
            .map(|frame| {
                frame
                    .iter()
                    .copied()
                    .fold(0.0_f32, |sum, sample| sum + sample.to_sample::<f32>())
                    / divisor
            })
            .collect()
    }

    /// Fill a device output slice from mono samples, duplicating mono to every
    /// channel. Missing samples stay at silence rather than repeating stale
    /// audio from the previous callback.
    fn from_mono<T>(data: &mut [T], mono: &[f32], channels: usize)
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        for (index, sample) in data.iter_mut().enumerate() {
            let source = index
                .checked_div(channels)
                .and_then(|mono_index| mono.get(mono_index))
                .copied()
                .unwrap_or(0.0);
            *sample = T::from_sample(source);
        }
    }

    fn stream_error_callback(
        sink: Arc<Mutex<Option<String>>>,
    ) -> impl FnMut(cpal::Error) + Send + 'static {
        move |error| {
            let mut slot = lock(&sink);
            if slot.is_none() {
                *slot = Some(format!("Audio device error: {error}"));
            }
        }
    }

    fn build_input_stream(
        device: &cpal::Device,
        config: &cpal::SupportedStreamConfig,
        capture: Arc<Mutex<CaptureState>>,
        errors: Arc<Mutex<Option<String>>>,
    ) -> Result<cpal::Stream, String> {
        let stream_config = config.config();
        let channels = usize::from(config.channels());
        let map_error = |error: cpal::Error| format!("Could not open microphone: {error}");
        match config.sample_format() {
            cpal::SampleFormat::I16 => device
                .build_input_stream::<i16, _, _>(
                    stream_config,
                    move |data, _| lock(&capture).push(data, channels),
                    stream_error_callback(errors),
                    None,
                )
                .map_err(map_error),
            cpal::SampleFormat::F32 => device
                .build_input_stream::<f32, _, _>(
                    stream_config,
                    move |data, _| lock(&capture).push(data, channels),
                    stream_error_callback(errors),
                    None,
                )
                .map_err(map_error),
            other => Err(format!("Unsupported microphone sample format: {other:?}")),
        }
    }

    fn build_output_stream(
        device: &cpal::Device,
        config: &cpal::SupportedStreamConfig,
        playback: Arc<Mutex<PlaybackState>>,
        errors: Arc<Mutex<Option<String>>>,
    ) -> Result<cpal::Stream, String> {
        let stream_config = config.config();
        let channels = usize::from(config.channels());
        let map_error = |error: cpal::Error| format!("Could not open speaker: {error}");
        match config.sample_format() {
            cpal::SampleFormat::I16 => device
                .build_output_stream::<i16, _, _>(
                    stream_config,
                    move |data, _| {
                        let frames = data.len().checked_div(channels).unwrap_or(0);
                        let mono = lock(&playback).take_mono(frames);
                        from_mono(data, &mono, channels);
                    },
                    stream_error_callback(errors),
                    None,
                )
                .map_err(map_error),
            cpal::SampleFormat::F32 => device
                .build_output_stream::<f32, _, _>(
                    stream_config,
                    move |data, _| {
                        let frames = data.len().checked_div(channels).unwrap_or(0);
                        let mono = lock(&playback).take_mono(frames);
                        from_mono(data, &mono, channels);
                    },
                    stream_error_callback(errors),
                    None,
                )
                .map_err(map_error),
            other => Err(format!("Unsupported speaker sample format: {other:?}")),
        }
    }

    /// Own both streams on one dedicated thread. Frames are pulled from the
    /// capture queue every 20 ms; playback is filled by `cpal`'s own callback.
    fn run_audio(
        call_id: &str,
        swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
        control_rx: &std::sync::mpsc::Receiver<Control>,
    ) -> Result<(), String> {
        let host = cpal::default_host();
        let input = host
            .default_input_device()
            .ok_or_else(|| "No microphone found (check OS audio settings)".to_owned())?;
        let output = host
            .default_output_device()
            .ok_or_else(|| "No speaker found (check OS audio settings)".to_owned())?;
        let input_config = input
            .default_input_config()
            .map_err(|error| format!("Microphone configuration unavailable: {error}"))?;
        let output_config = output
            .default_output_config()
            .map_err(|error| format!("Speaker configuration unavailable: {error}"))?;

        let capture = Arc::new(Mutex::new(CaptureState::new(input_config.sample_rate())));
        let playback = Arc::new(Mutex::new(PlaybackState::new(output_config.sample_rate())));
        let errors: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let input_stream = build_input_stream(
            &input,
            &input_config,
            Arc::clone(&capture),
            Arc::clone(&errors),
        )?;
        let playback_for_control = Arc::clone(&playback);
        let output_stream = build_output_stream(&output, &output_config, playback, errors.clone())?;
        input_stream
            .play()
            .map_err(|error| format!("Could not start microphone: {error}"))?;
        output_stream
            .play()
            .map_err(|error| format!("Could not start speaker: {error}"))?;

        loop {
            let stream_error = lock(&errors).take();
            if let Some(error) = stream_error {
                return Err(error);
            }
            let mut stopping = false;
            while let Ok(control) = control_rx.try_recv() {
                match control {
                    Control::Stop => {
                        stopping = true;
                        break;
                    }
                    Control::Play(pcm) => lock(&playback_for_control).enqueue(&pcm),
                }
            }
            if stopping {
                break;
            }

            let frames = {
                let mut state = lock(&capture);
                let mut frames = Vec::new();
                while let Some(frame) = state.take_frame() {
                    frames.push(frame);
                }
                frames
            };
            for pcm in frames {
                let command = SwarmCommand::SendCallMedia {
                    call_id: call_id.to_owned(),
                    pcm,
                };
                if swarm_cmd_tx.try_send(command).is_err() {
                    break;
                }
            }
            std::thread::sleep(FRAME_INTERVAL);
        }

        drop(input_stream);
        drop(output_stream);
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::{RateConverter, to_mono};

        fn assert_close(actual: f32, expected: f32) {
            assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
        }

        #[test]
        fn resamples_down_to_wire_rate() {
            let mut converter = RateConverter::new(48_000, 16_000);
            let input = vec![1.0_f32; 480];
            let mut output = Vec::new();
            converter.push(&input, &mut output);
            assert!((140..180).contains(&output.len()), "{}", output.len());
            assert!(output.iter().all(|sample| (*sample - 1.0).abs() < 0.0001));
        }

        #[test]
        fn resamples_up_to_device_rate() {
            let mut converter = RateConverter::new(16_000, 48_000);
            let input = vec![1.0_f32; 160];
            let mut output = Vec::new();
            converter.push(&input, &mut output);
            assert!((440..500).contains(&output.len()), "{}", output.len());
            assert!(output.iter().all(|sample| (*sample - 1.0).abs() < 0.0001));
        }

        #[test]
        fn averages_interleaved_channels() {
            let samples = [1_i16, -1, 0, 2];
            let mono = to_mono(&samples, 2);
            assert_eq!(mono.len(), 2);
            assert_close(mono[0], 0.0);
            assert_close(mono[1], 1.0 / 32_767.0);
        }
    }
}

#[cfg(not(feature = "tui-audio"))]
mod disabled {
    use p2p_app::SwarmCommand;
    use tokio::sync::mpsc;

    /// No-op stand-in used by every build without the `tui-audio` feature.
    /// Keeping the same surface lets the TUI call sites stay unconditional.
    #[derive(Clone, Copy, Debug)]
    pub struct TuiAudioHandle {
        reason: &'static str,
    }

    impl Default for TuiAudioHandle {
        fn default() -> Self {
            Self {
                reason: "tui-audio feature disabled",
            }
        }
    }

    impl TuiAudioHandle {
        pub const fn start(
            &self,
            _call_id: &str,
            _peer_id: &str,
            _swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
        ) {
            let _ = self.reason;
        }

        pub const fn stop(&self) {
            let _ = self.reason;
        }

        pub const fn play(&self, _pcm: &[i16]) {
            let _ = self.reason;
        }

        #[must_use]
        pub const fn take_error(&self) -> Option<String> {
            let _ = self.reason;
            None
        }
    }
}

#[cfg(not(feature = "tui-audio"))]
pub use disabled::TuiAudioHandle;
#[cfg(feature = "tui-audio")]
pub use enabled::TuiAudioHandle;
