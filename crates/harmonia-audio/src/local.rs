use std::path::Path;
use std::sync::Arc;
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use parking_lot::Mutex;
use tracing::{info, error, debug};

use harmonia_core::models::PlaybackState;
use crate::{PlaybackEngine, PlaybackEvent};

/// Commands sent from the UI thread to the audio thread.
enum AudioCommand {
    Load(String),
    Play,
    Pause,
    Stop,
    Seek(u64),
    SetVolume(f32),
    Shutdown,
}

/// Shared state accessible from both UI and audio threads.
struct SharedState {
    state: PlaybackState,
    position_ms: u64,
    duration_ms: u64,
    volume: f32,
}

/// Local file playback engine using symphonia + cpal.
pub struct LocalPlayback {
    shared: Arc<Mutex<SharedState>>,
    cmd_tx: Sender<AudioCommand>,
    event_rx: Receiver<PlaybackEvent>,
    _audio_thread: std::thread::JoinHandle<()>,
}

impl LocalPlayback {
    pub fn new(event_tx: Sender<PlaybackEvent>) -> Result<Self> {
        let (cmd_tx, cmd_rx) = bounded::<AudioCommand>(32);
        let event_tx_clone = event_tx.clone();

        let shared = Arc::new(Mutex::new(SharedState {
            state: PlaybackState::Stopped,
            position_ms: 0,
            duration_ms: 0,
            volume: 0.8,
        }));
        let shared_clone = shared.clone();

        let audio_thread = std::thread::Builder::new()
            .name("harmonia-audio".to_string())
            .spawn(move || {
                audio_thread_main(cmd_rx, event_tx_clone, shared_clone);
            })?;

        Ok(Self {
            shared,
            cmd_tx,
            event_rx: {
                let (_, rx) = bounded(1);
                rx
            },
            _audio_thread: audio_thread,
        })
    }

    /// Get the event receiver to poll for playback events.
    pub fn events(&self) -> &Receiver<PlaybackEvent> {
        &self.event_rx
    }
}

impl PlaybackEngine for LocalPlayback {
    fn load(&mut self, path: &str) -> Result<()> {
        let _ = self.cmd_tx.send(AudioCommand::Load(path.to_string()));
        Ok(())
    }

    fn play(&mut self) {
        let _ = self.cmd_tx.send(AudioCommand::Play);
    }

    fn pause(&mut self) {
        let _ = self.cmd_tx.send(AudioCommand::Pause);
    }

    fn stop(&mut self) {
        let _ = self.cmd_tx.send(AudioCommand::Stop);
    }

    fn seek(&mut self, position_ms: u64) {
        let _ = self.cmd_tx.send(AudioCommand::Seek(position_ms));
    }

    fn volume(&self) -> f32 {
        self.shared.lock().volume
    }

    fn set_volume(&mut self, vol: f32) {
        self.shared.lock().volume = vol.clamp(0.0, 1.0);
        let _ = self.cmd_tx.send(AudioCommand::SetVolume(vol.clamp(0.0, 1.0)));
    }

    fn position_ms(&self) -> u64 {
        self.shared.lock().position_ms
    }

    fn duration_ms(&self) -> u64 {
        self.shared.lock().duration_ms
    }

    fn state(&self) -> PlaybackState {
        self.shared.lock().state
    }
}

impl Drop for LocalPlayback {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(AudioCommand::Shutdown);
    }
}

/// The audio thread: decodes with symphonia, outputs with cpal.
fn audio_thread_main(
    cmd_rx: Receiver<AudioCommand>,
    event_tx: Sender<PlaybackEvent>,
    shared: Arc<Mutex<SharedState>>,
) {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    use symphonia::core::audio::Signal;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        None => {
            error!("No audio output device found");
            let _ = event_tx.send(PlaybackEvent::Error("No audio output device".into()));
            return;
        }
    };

    info!("Using audio device: {:?}", device.name());

    // Ring buffer for decoded samples
    let sample_buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(48000 * 2)));
    let sample_buf_writer = sample_buf.clone();
    let volume = Arc::new(Mutex::new(0.8f32));
    let volume_reader = volume.clone();
    let is_playing = Arc::new(Mutex::new(false));
    let is_playing_reader = is_playing.clone();

    // Build cpal output stream
    let config = cpal::StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(44100),
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let playing = *is_playing_reader.lock();
            let vol = *volume_reader.lock();
            if !playing {
                data.fill(0.0);
                return;
            }
            let mut buf = sample_buf.lock();
            let available = buf.len().min(data.len());
            if available > 0 {
                for (out, sample) in data[..available].iter_mut().zip(buf.drain(..available)) {
                    *out = sample * vol;
                }
            }
            // Fill remainder with silence
            data[available..].fill(0.0);
        },
        |err| {
            error!("cpal stream error: {err}");
        },
        None,
    );

    let stream = match stream {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to build audio stream: {e}");
            let _ = event_tx.send(PlaybackEvent::Error(format!("Audio stream error: {e}")));
            return;
        }
    };

    if let Err(e) = stream.play() {
        error!("Failed to start audio stream: {e}");
        return;
    }

    // Decoder state
    let mut current_format: Option<Box<dyn symphonia::core::formats::FormatReader>> = None;
    let mut current_decoder: Option<Box<dyn symphonia::core::codecs::Decoder>> = None;
    let mut current_track_id: u32 = 0;
    let mut sample_rate: u32 = 44100;

    loop {
        // Process commands (non-blocking if we're decoding)
        let timeout = if *is_playing.lock() && current_format.is_some() {
            std::time::Duration::from_millis(1)
        } else {
            std::time::Duration::from_millis(50)
        };

        if let Ok(cmd) = cmd_rx.recv_timeout(timeout) {
            match cmd {
                AudioCommand::Load(path) => {
                    debug!("Loading: {path}");
                    *is_playing.lock() = false;
                    sample_buf_writer.lock().clear();

                    match open_audio_file(&path) {
                        Ok((format, decoder, track_id, sr, dur_ms)) => {
                            current_format = Some(format);
                            current_decoder = Some(decoder);
                            current_track_id = track_id;
                            sample_rate = sr;
                            {
                                let mut s = shared.lock();
                                s.duration_ms = dur_ms;
                                s.position_ms = 0;
                                s.state = PlaybackState::Paused;
                            }
                            let _ = event_tx.send(PlaybackEvent::TrackLoaded { duration_ms: dur_ms });
                            let _ = event_tx.send(PlaybackEvent::StateChanged(PlaybackState::Paused));
                        }
                        Err(e) => {
                            error!("Failed to load {path}: {e}");
                            let _ = event_tx.send(PlaybackEvent::Error(format!("Load error: {e}")));
                        }
                    }
                }
                AudioCommand::Play => {
                    if current_format.is_some() {
                        *is_playing.lock() = true;
                        shared.lock().state = PlaybackState::Playing;
                        let _ = event_tx.send(PlaybackEvent::StateChanged(PlaybackState::Playing));
                    }
                }
                AudioCommand::Pause => {
                    *is_playing.lock() = false;
                    shared.lock().state = PlaybackState::Paused;
                    let _ = event_tx.send(PlaybackEvent::StateChanged(PlaybackState::Paused));
                }
                AudioCommand::Stop => {
                    *is_playing.lock() = false;
                    sample_buf_writer.lock().clear();
                    current_format = None;
                    current_decoder = None;
                    {
                        let mut s = shared.lock();
                        s.state = PlaybackState::Stopped;
                        s.position_ms = 0;
                    }
                    let _ = event_tx.send(PlaybackEvent::StateChanged(PlaybackState::Stopped));
                }
                AudioCommand::Seek(ms) => {
                    if let Some(ref mut format) = current_format {
                        let ts = symphonia::core::formats::SeekTo::Time {
                            time: symphonia::core::units::Time {
                                seconds: ms / 1000,
                                frac: (ms % 1000) as f64 / 1000.0,
                            },
                            track_id: Some(current_track_id),
                        };
                        sample_buf_writer.lock().clear();
                        if let Err(e) = format.seek(symphonia::core::formats::SeekMode::Coarse, ts) {
                            error!("Seek error: {e}");
                        } else {
                            shared.lock().position_ms = ms;
                            let _ = event_tx.send(PlaybackEvent::PositionChanged(ms));
                        }
                    }
                }
                AudioCommand::SetVolume(v) => {
                    *volume.lock() = v;
                }
                AudioCommand::Shutdown => {
                    *is_playing.lock() = false;
                    info!("Audio thread shutting down");
                    break;
                }
            }
        }

        // Decode more samples if playing and buffer is low
        if *is_playing.lock() {
            if let (Some(ref mut format), Some(ref mut decoder)) =
                (&mut current_format, &mut current_decoder)
            {
                let buf_len = sample_buf_writer.lock().len();
                // Keep ~200ms of audio buffered
                let target_samples = (sample_rate as usize) * 2 * 200 / 1000;
                if buf_len < target_samples {
                    match decode_next_packet(format.as_mut(), decoder.as_mut(), current_track_id) {
                        Ok(Some(samples)) => {
                            // Update position
                            let position_samples = {
                                let buf = sample_buf_writer.lock();
                                buf.len() / 2
                            };
                            let pos_ms = (position_samples as u64 * 1000) / sample_rate as u64;
                            shared.lock().position_ms += pos_ms.min(50); // approximate

                            sample_buf_writer.lock().extend_from_slice(&samples);
                        }
                        Ok(None) => {
                            // Track finished — wait for buffer to drain
                            if sample_buf_writer.lock().is_empty() {
                                *is_playing.lock() = false;
                                shared.lock().state = PlaybackState::Stopped;
                                let _ = event_tx.send(PlaybackEvent::TrackFinished);
                                let _ = event_tx.send(PlaybackEvent::StateChanged(PlaybackState::Stopped));
                                current_format = None;
                                current_decoder = None;
                            }
                        }
                        Err(e) => {
                            error!("Decode error: {e}");
                            let _ = event_tx.send(PlaybackEvent::Error(format!("Decode error: {e}")));
                        }
                    }
                }
            }
        }
    }

    drop(stream);
}

/// Open an audio file and return the format reader, decoder, and metadata.
fn open_audio_file(path: &str) -> Result<(
    Box<dyn symphonia::core::formats::FormatReader>,
    Box<dyn symphonia::core::codecs::Decoder>,
    u32,  // track_id
    u32,  // sample_rate
    u64,  // duration_ms
)> {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &MetadataOptions::default())?;

    let format = probed.format;
    let track = format.default_track()
        .ok_or_else(|| anyhow::anyhow!("No audio track found"))?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);

    // Calculate duration
    let duration_ms = if let Some(n_frames) = track.codec_params.n_frames {
        (n_frames as u64 * 1000) / sample_rate as u64
    } else {
        0
    };

    let decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())?;

    Ok((format, decoder, track_id, sample_rate, duration_ms))
}

/// Decode the next packet and return interleaved f32 samples (stereo).
fn decode_next_packet(
    format: &mut dyn symphonia::core::formats::FormatReader,
    decoder: &mut dyn symphonia::core::codecs::Decoder,
    track_id: u32,
) -> Result<Option<Vec<f32>>> {
    use symphonia::core::audio::Signal;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                return Ok(None); // End of file
            }
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder.decode(&packet)?;
        let spec = *decoded.spec();
        let channels = spec.channels.count();
        let frames = decoded.frames();

        // Convert to interleaved stereo f32
        let mut samples = Vec::with_capacity(frames * 2);
        let mut sample_buf = symphonia::core::audio::SampleBuffer::<f32>::new(
            frames as u64,
            spec,
        );
        sample_buf.copy_interleaved_ref(decoded);
        let interleaved = sample_buf.samples();

        if channels == 1 {
            // Mono → stereo
            for &s in interleaved {
                samples.push(s);
                samples.push(s);
            }
        } else if channels == 2 {
            samples.extend_from_slice(interleaved);
        } else {
            // Downmix to stereo (simple: take first two channels)
            for frame in interleaved.chunks(channels) {
                samples.push(frame.first().copied().unwrap_or(0.0));
                samples.push(frame.get(1).copied().unwrap_or(0.0));
            }
        }

        return Ok(Some(samples));
    }
}
