pub mod local;
pub mod router;
pub mod spotify_player;

use harmonia_core::models::PlaybackState;

/// Events emitted by the playback engine.
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    StateChanged(PlaybackState),
    PositionChanged(u64),       // ms
    TrackFinished,
    TrackLoaded { duration_ms: u64 },
    Error(String),
}

/// Unified playback control interface.
pub trait PlaybackEngine: Send {
    fn load(&mut self, path: &str) -> anyhow::Result<()>;
    fn play(&mut self);
    fn pause(&mut self);
    fn stop(&mut self);
    fn seek(&mut self, position_ms: u64);
    fn volume(&self) -> f32;
    fn set_volume(&mut self, vol: f32);
    fn position_ms(&self) -> u64;
    fn duration_ms(&self) -> u64;
    fn state(&self) -> PlaybackState;
}
