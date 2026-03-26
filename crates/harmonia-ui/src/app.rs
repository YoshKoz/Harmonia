use gpui::*;
use std::sync::Arc;
use parking_lot::Mutex;

use harmonia_core::config::Theme;
use harmonia_core::db::Database;
use harmonia_core::models::*;
use harmonia_audio::router::AudioRouter;
use crate::theme::HarmoniaTheme;

/// Which main view is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Library,
    Albums,
    AlbumDetail,
    Artists,
    Playlists,
    PlaylistDetail,
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
    pub search_results: Vec<UnifiedTrack>,
    pub tracks_cache: Vec<UnifiedTrack>,
    pub albums_cache: Vec<Album>,
    pub playlists_cache: Vec<Playlist>,
    pub selected_album: Option<Album>,
    pub album_tracks_cache: Vec<UnifiedTrack>,
    pub selected_playlist: Option<Playlist>,
    pub playlist_tracks_cache: Vec<UnifiedTrack>,
}

impl AppState {
    pub fn new(db: Database, audio: AudioRouter, theme: Theme) -> Self {
        Self {
            db,
            audio: Arc::new(Mutex::new(audio)),
            theme: match theme {
                Theme::Dark => HarmoniaTheme::dark(),
                Theme::Light => HarmoniaTheme::light(),
            },
            active_view: ActiveView::Library,
            current_track: None,
            queue: Vec::new(),
            queue_index: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            tracks_cache: Vec::new(),
            albums_cache: Vec::new(),
            playlists_cache: Vec::new(),
            selected_album: None,
            album_tracks_cache: Vec::new(),
            selected_playlist: None,
            playlist_tracks_cache: Vec::new(),
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

    /// Navigate into album detail view.
    pub fn navigate_to_album(&mut self, index: usize) {
        if let Some(album) = self.albums_cache.get(index).cloned() {
            if let Ok(tracks) = self.db.get_album_tracks(&album.title, &album.artist) {
                self.album_tracks_cache = tracks;
            }
            self.selected_album = Some(album);
            self.active_view = ActiveView::AlbumDetail;
        }
    }

    /// Navigate into playlist detail view.
    pub fn navigate_to_playlist(&mut self, index: usize) {
        if let Some(playlist) = self.playlists_cache.get(index).cloned() {
            if let Ok(tracks) = self.db.get_playlist_tracks(playlist.id) {
                self.playlist_tracks_cache = tracks;
            }
            self.selected_playlist = Some(playlist);
            self.active_view = ActiveView::PlaylistDetail;
        }
    }

    /// Perform a search query.
    pub fn perform_search(&mut self, query: &str) {
        self.search_query = query.to_string();
        if query.is_empty() {
            self.search_results.clear();
        } else if let Ok(results) = self.db.search_tracks(query) {
            self.search_results = results;
        }
    }
}
