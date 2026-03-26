use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Where a track originates from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackSource {
    Local(PathBuf),
    Spotify(String), // Spotify URI e.g. "spotify:track:xyz"
}

/// Playback state of the audio engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    Loading,
}

/// Core track representation that unifies local and Spotify tracks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTrack {
    pub id: i64,
    pub source: TrackSource,
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub genre: String,
    pub year: Option<i32>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub duration_ms: u64,
    pub artwork_hash: Option<String>,
    pub rating: Option<u8>,
    pub play_count: u32,
    pub date_added: i64, // unix timestamp
    pub last_played: Option<i64>,
}

impl UnifiedTrack {
    pub fn display_artist(&self) -> &str {
        if self.artist.is_empty() {
            "Unknown Artist"
        } else {
            &self.artist
        }
    }

    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            "Unknown Title"
        } else {
            &self.title
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self.source, TrackSource::Local(_))
    }

    pub fn is_spotify(&self) -> bool {
        matches!(self.source, TrackSource::Spotify(_))
    }
}

/// Album representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub year: Option<i32>,
    pub artwork_hash: Option<String>,
    pub track_count: u32,
}

/// Playlist that can contain both local and Spotify tracks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub source: PlaylistSource,
    pub is_smart: bool,
    pub smart_query: Option<SmartPlaylistQuery>,
    pub track_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaylistSource {
    Local,
    Spotify(String), // Spotify playlist ID
    Mixed,
}

/// Smart playlist query definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartPlaylistQuery {
    pub rules: Vec<SmartRule>,
    pub match_all: bool, // true = AND, false = OR
    pub limit: Option<u32>,
    pub sort_field: Option<String>,
    pub sort_desc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartRule {
    pub field: String,
    pub op: SmartOp,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SmartOp {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
}

/// Cached artwork entry.
#[derive(Debug, Clone)]
pub struct ArtworkEntry {
    pub hash: String,
    pub data: Vec<u8>,
    pub mime: String,
}

/// Queue item for the unified playback queue.
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub track: UnifiedTrack,
    pub queue_position: usize,
}

/// Lyrics data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lyrics {
    pub track_id: i64,
    pub synced: bool,
    pub lines: Vec<LyricLine>,
    pub plain_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricLine {
    pub timestamp_ms: u64,
    pub text: String,
}
