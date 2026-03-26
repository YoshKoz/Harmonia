use anyhow::Result;
use crossbeam_channel::{Sender, Receiver, bounded};
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::info;

use harmonia_core::models::{PlaybackState, TrackSource, UnifiedTrack};
use crate::{PlaybackEngine, PlaybackEvent};
use crate::local::LocalPlayback;

/// The AudioRouter dispatches playback to the correct engine based on track source.
pub struct AudioRouter {
    local_engine: LocalPlayback,
    // spotify_engine will be added in Phase 3
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
            event_tx,
            event_rx,
            current_source: None,
        })
    }

    /// Load and auto-play a unified track.
    pub fn load_track(&mut self, track: &UnifiedTrack) -> Result<()> {
        match &track.source {
            TrackSource::Local(path) => {
                let path_str = path.to_string_lossy().to_string();
                self.local_engine.load(&path_str)?;
                self.local_engine.play();
                self.current_source = Some(track.source.clone());
            }
            TrackSource::Spotify(_uri) => {
                // Phase 3: delegate to spotify engine
                info!("Spotify playback not yet implemented");
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
            _ => {}
        }
    }

    pub fn pause(&mut self) {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.pause(),
            _ => {}
        }
    }

    pub fn stop(&mut self) {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.stop(),
            _ => {}
        }
    }

    pub fn seek(&mut self, position_ms: u64) {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.seek(position_ms),
            _ => {}
        }
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.local_engine.set_volume(vol);
    }

    pub fn volume(&self) -> f32 {
        self.local_engine.volume()
    }

    pub fn position_ms(&self) -> u64 {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.position_ms(),
            _ => 0,
        }
    }

    pub fn duration_ms(&self) -> u64 {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.duration_ms(),
            _ => 0,
        }
    }

    pub fn state(&self) -> PlaybackState {
        match &self.current_source {
            Some(TrackSource::Local(_)) => self.local_engine.state(),
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
