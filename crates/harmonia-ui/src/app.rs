use gpui::*;
use std::sync::Arc;
use parking_lot::Mutex;

use harmonia_core::db::Database;
use harmonia_core::models::*;
use harmonia_audio::router::AudioRouter;
use crate::theme::HarmoniaTheme;

/// Which main view is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Library,
    Albums,
    Artists,
    Playlists,
    Search,
    NowPlaying,
    Settings,
}

/// The root application state accessible globally.
pub struct AppState {
    pub db: Database,
    pub audio: Arc<Mutex<AudioRouter>>,
    pub theme: HarmoniaTheme,
    pub active_view: ActiveView,
    pub current_track: Option<UnifiedTrack>,
    pub queue: Vec<UnifiedTrack>,
    pub queue_index: usize,
    pub search_query: String,
    pub tracks_cache: Vec<UnifiedTrack>,
    pub albums_cache: Vec<Album>,
    pub playlists_cache: Vec<Playlist>,
}

impl AppState {
    pub fn new(db: Database, audio: AudioRouter) -> Self {
        Self {
            db,
            audio: Arc::new(Mutex::new(audio)),
            theme: HarmoniaTheme::dark(),
            active_view: ActiveView::Library,
            current_track: None,
            queue: Vec::new(),
            queue_index: 0,
            search_query: String::new(),
            tracks_cache: Vec::new(),
            albums_cache: Vec::new(),
            playlists_cache: Vec::new(),
        }
    }

    /// Refresh in-memory caches from DB.
    pub fn refresh_library(&mut self) {
        if let Ok(tracks) = self.db.get_tracks(None) {
            self.tracks_cache = tracks;
        }
        if let Ok(albums) = self.db.get_albums() {
            self.albums_cache = albums;
        }
        if let Ok(playlists) = self.db.get_playlists() {
            self.playlists_cache = playlists;
        }
    }

    /// Play a track from the queue at the given index.
    pub fn play_track_at(&mut self, index: usize) {
        if let Some(track) = self.queue.get(index).cloned() {
            self.queue_index = index;
            self.current_track = Some(track.clone());
            let mut audio = self.audio.lock();
            if let Err(e) = audio.load_track(&track) {
                tracing::error!("Failed to play track: {e}");
            }
        }
    }

    /// Play the next track in the queue.
    pub fn next_track(&mut self) {
        if self.queue_index + 1 < self.queue.len() {
            self.play_track_at(self.queue_index + 1);
        }
    }

    /// Play the previous track in the queue.
    pub fn prev_track(&mut self) {
        if self.queue_index > 0 {
            self.play_track_at(self.queue_index - 1);
        }
    }

    /// Set the queue to a list of tracks and start playing from index 0.
    pub fn play_tracks(&mut self, tracks: Vec<UnifiedTrack>, start_index: usize) {
        self.queue = tracks;
        self.play_track_at(start_index);
    }
}
