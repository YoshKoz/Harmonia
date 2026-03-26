use anyhow::Result;
use rspotify::prelude::*;
use rspotify::AuthCodePkceSpotify;
use rspotify::model::{SimplifiedPlaylist, SavedAlbum, SavedTrack, FullTrack, PlaylistId, SearchType, SearchResult};
use tracing::debug;

/// High-level Spotify API client wrapping rspotify.
pub struct SpotifyClient {
    api: AuthCodePkceSpotify,
}

/// A simplified Spotify track for use in the unified library.
#[derive(Debug, Clone)]
pub struct SpotifyTrackInfo {
    pub uri: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub image_url: Option<String>,
}

/// A simplified Spotify playlist.
#[derive(Debug, Clone)]
pub struct SpotifyPlaylistInfo {
    pub id: String,
    pub name: String,
    pub track_count: u32,
    pub image_url: Option<String>,
}

/// A simplified Spotify album.
#[derive(Debug, Clone)]
pub struct SpotifyAlbumInfo {
    pub uri: String,
    pub name: String,
    pub artist: String,
    pub image_url: Option<String>,
    pub year: Option<i32>,
}

impl SpotifyClient {
    pub fn new(api: AuthCodePkceSpotify) -> Self {
        Self { api }
    }

    /// Fetch the user's playlists.
    pub async fn get_user_playlists(&self) -> Result<Vec<SpotifyPlaylistInfo>> {
        let mut playlists = Vec::new();
        let mut offset = 0;
        let limit = 50;

        loop {
            let page = self.api.current_user_playlists_manual(Some(limit), Some(offset)).await?;
            for item in &page.items {
                playlists.push(SpotifyPlaylistInfo {
                    id: item.id.to_string(),
                    name: item.name.clone(),
                    track_count: item.tracks.total,
                    image_url: item.images.first().map(|i| i.url.clone()),
                });
            }
            if page.next.is_none() {
                break;
            }
            offset += limit;
        }

        debug!("Fetched {} Spotify playlists", playlists.len());
        Ok(playlists)
    }

    /// Fetch tracks in a specific playlist.
    pub async fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<SpotifyTrackInfo>> {
        let mut tracks = Vec::new();
        let mut offset = 0;
        let limit = 100;
        let pid = PlaylistId::from_id(playlist_id)?;

        loop {
            let page = self.api.playlist_items_manual(&pid, None, None, Some(limit), Some(offset)).await?;
            for item in &page.items {
                if let Some(rspotify::model::PlayableItem::Track(track)) = &item.track {
                    tracks.push(full_track_to_info(track));
                }
            }
            if page.next.is_none() {
                break;
            }
            offset += limit;
        }

        Ok(tracks)
    }

    /// Fetch user's saved albums.
    pub async fn get_saved_albums(&self) -> Result<Vec<SpotifyAlbumInfo>> {
        let mut albums = Vec::new();
        let mut offset = 0;
        let limit = 50;

        loop {
            let page = self.api.current_user_saved_albums_manual(None, Some(limit), Some(offset)).await?;
            for saved in &page.items {
                let album = &saved.album;
                albums.push(SpotifyAlbumInfo {
                    uri: album.id.as_ref().map(|id| id.to_string()).unwrap_or_default(),
                    name: album.name.clone(),
                    artist: album.artists.first().map(|a| a.name.clone()).unwrap_or_default(),
                    image_url: album.images.first().map(|i| i.url.clone()),
                    year: album.release_date.as_ref().and_then(|d| d[..4].parse().ok()),
                });
            }
            if page.next.is_none() {
                break;
            }
            offset += limit;
        }

        debug!("Fetched {} saved Spotify albums", albums.len());
        Ok(albums)
    }

    /// Fetch user's saved/liked tracks.
    pub async fn get_saved_tracks(&self) -> Result<Vec<SpotifyTrackInfo>> {
        let mut tracks = Vec::new();
        let mut offset = 0;
        let limit = 50;

        loop {
            let page = self.api.current_user_saved_tracks_manual(None, Some(limit), Some(offset)).await?;
            for saved in &page.items {
                tracks.push(full_track_to_info(&saved.track));
            }
            if page.next.is_none() {
                break;
            }
            offset += limit;
        }

        debug!("Fetched {} saved Spotify tracks", tracks.len());
        Ok(tracks)
    }

    /// Search Spotify for tracks.
    pub async fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<SpotifyTrackInfo>> {
        let results = self.api.search(query, SearchType::Track, None, None, Some(limit), None).await?;
        let mut tracks = Vec::new();
        if let SearchResult::Tracks(page) = results {
            for track in &page.items {
                tracks.push(full_track_to_info(track));
            }
        }
        Ok(tracks)
    }
}

fn full_track_to_info(track: &FullTrack) -> SpotifyTrackInfo {
    SpotifyTrackInfo {
        uri: track.id.as_ref().map(|id| format!("spotify:track:{}", id.to_string())).unwrap_or_default(),
        title: track.name.clone(),
        artist: track.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
        album: track.album.name.clone(),
        duration_ms: track.duration.num_milliseconds() as u64,
        image_url: track.album.images.first().map(|i| i.url.clone()),
    }
}
