use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::path::PathBuf;
use tracing::{info, warn};

use crate::local::LocalPlayback;
use crate::spotify_player::SpotifyPlayer;
use crate::{PlaybackEngine, PlaybackEvent};
use harmonia_core::models::{PlaybackState, TrackSource, UnifiedTrack};

/// Dispatches playback to the correct engine based on track source.
pub struct AudioRouter {
    local_engine: LocalPlayback,
    spotify_engine: Option<SpotifyPlayer>,
    event_tx: Sender<PlaybackEvent>,
    event_rx: Receiver<PlaybackEvent>,
    current_source: Option<TrackSource>,
}

impl AudioRouter {
    pub fn new() -> Result<Self> {
        let (event_tx, event_rx) = bounded(256);
        let local_engine = LocalPlayback::new(event_tx.clone())?;
        Ok(Self {
            local_engine,
            spotify_engine: None,
            event_tx,
            event_rx,
            current_source: None,
        })
    }

    /// Enable Spotify streaming using librespot's credential cache.
    /// On first use, credentials are absent — call `spotify_login` once.
    pub fn enable_spotify(&mut self, cache_dir: PathBuf) {
        let player = SpotifyPlayer::new(cache_dir, self.event_tx.clone());
        self.spotify_engine = Some(player);
        info!("Spotify streaming enabled");
    }

    /// One-time login.  Password is used once and immediately dropped;
    /// librespot caches an encrypted blob for all future launches.
    pub fn spotify_login(&mut self, username: String, password: String) {
        if let Some(sp) = &self.spotify_engine {
            sp.login(username, password);
        } else {
            warn!("spotify_login called but Spotify engine not initialised");
        }
    }

    pub fn has_spotify(&self) -> bool {
        self.spotify_engine.is_some()
    }

    /// Load and auto-play a track.
    pub fn load_track(&mut self, track: &UnifiedTrack) -> Result<()> {
        match &track.source {
            TrackSource::Local(path) => {
                let path_str = path.to_string_lossy().to_string();
                self.local_engine.load(&path_str)?;
                self.local_engine.play();
                self.current_source = Some(track.source.clone());
            }
            TrackSource::Spotify(uri) => {
                if let Some(sp) = &mut self.spotify_engine {
                    sp.load(uri)?;
                    self.current_source = Some(track.source.clone());
                } else {
                    warn!("Spotify track requested but Spotify not enabled");
                }
            }
        }
        Ok(())
    }

    /// Get the event receiver for UI polling.
    pub fn event_rx(&self) -> &Receiver<PlaybackEvent> {
        &self.event_rx
    }

    pub fn play(&mut self) {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.play(),
            Some(TrackSource::Spotify(_)) => {
                if let Some(sp) = &mut self.spotify_engine {
                    sp.play();
                }
            }
            _ => {}
        }
    }

    pub fn pause(&mut self) {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.pause(),
            Some(TrackSource::Spotify(_)) => {
                if let Some(sp) = &mut self.spotify_engine {
                    sp.pause();
                }
            }
            _ => {}
        }
    }

    pub fn stop(&mut self) {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.stop(),
            Some(TrackSource::Spotify(_)) => {
                if let Some(sp) = &mut self.spotify_engine {
                    sp.stop();
                }
            }
            _ => {}
        }
    }

    pub fn seek(&mut self, position_ms: u64) {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.seek(position_ms),
            Some(TrackSource::Spotify(_)) => {
                if let Some(sp) = &mut self.spotify_engine {
                    sp.seek(position_ms);
                }
            }
            _ => {}
        }
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.local_engine.set_volume(vol);
        if let Some(sp) = &mut self.spotify_engine {
            sp.set_volume(vol);
        }
    }

    pub fn volume(&self) -> f32 {
        self.local_engine.volume()
    }

    pub fn position_ms(&self) -> u64 {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.position_ms(),
            Some(TrackSource::Spotify(_)) => self
                .spotify_engine
                .as_ref()
                .map(|sp| sp.position_ms())
                .unwrap_or(0),
            _ => 0,
        }
    }

    pub fn duration_ms(&self) -> u64 {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.duration_ms(),
            Some(TrackSource::Spotify(_)) => self
                .spotify_engine
                .as_ref()
                .map(|sp| sp.duration_ms())
                .unwrap_or(0),
            _ => 0,
        }
    }

    pub fn state(&self) -> PlaybackState {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.state(),
            Some(TrackSource::Spotify(_)) => self
                .spotify_engine
                .as_ref()
                .map(|sp| sp.state())
                .unwrap_or(PlaybackState::Stopped),
            _ => PlaybackState::Stopped,
        }
    }

    pub fn toggle_play_pause(&mut self) {
        match self.state() {
            PlaybackState::Playing => self.pause(),
            PlaybackState::Paused => self.play(),
            _ => {}
        }
    }
}
