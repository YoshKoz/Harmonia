use anyhow::Result;
use harmonia_core::db::Database;
use tracing::{info, warn};

use crate::client::{SpotifyClient, SpotifyTrackInfo};

/// Synchronize Spotify library data into the local database.
pub async fn sync_spotify_library(client: &SpotifyClient, db: &Database) -> Result<SyncStats> {
    let mut stats = SyncStats::default();

    // Sync saved tracks
    info!("Syncing Spotify saved tracks...");
    match client.get_saved_tracks().await {
        Ok(tracks) => {
            for track in &tracks {
                match upsert_spotify_track(db, track).await {
                    Err(e) => {
                        warn!("Failed to sync track '{}': {e}", track.title);
                        stats.errors += 1;
                    }
                    Ok(_) => {
                        stats.tracks_synced += 1;
                    }
                }
            }
        }
        Err(e) => {
            warn!("Failed to fetch saved tracks: {e}");
            stats.errors += 1;
        }
    }

    // Sync playlists
    info!("Syncing Spotify playlists...");
    match client.get_user_playlists().await {
        Ok(playlists) => {
            for playlist in &playlists {
                stats.playlists_synced += 1;
                // Create or update playlist in DB
                match db.create_playlist(&playlist.name, "spotify") {
                    Ok(playlist_id) => {
                        // Fetch and sync playlist tracks
                        match client.get_playlist_tracks(&playlist.id).await {
                            Ok(tracks) => {
                                for track in &tracks {
                                    match upsert_spotify_track(db, track).await {
                                        Ok(track_id) => {
                                            if let Err(e) = db.add_track_to_playlist(playlist_id, track_id) {
                                                warn!("Failed to link track to playlist: {e}");
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Failed to sync playlist track '{}': {e}", track.title);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to fetch playlist tracks for '{}': {e}", playlist.name);
                                stats.errors += 1;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to create playlist '{}': {e}", playlist.name);
                        stats.errors += 1;
                    }
                }
            }
        }
        Err(e) => {
            warn!("Failed to fetch playlists: {e}");
            stats.errors += 1;
        }
    }

    // Rebuild albums index after sync
    db.rebuild_albums()?;

    info!(
        "Spotify sync complete: {} tracks, {} playlists, {} errors",
        stats.tracks_synced, stats.playlists_synced, stats.errors
    );
    Ok(stats)
}

async fn upsert_spotify_track(db: &Database, track: &SpotifyTrackInfo) -> Result<i64> {
    let artwork_hash = match &track.image_url {
        Some(url) => download_and_cache_artwork(db, url).await.ok(),
        None => None,
    };

    db.upsert_spotify_track(
        &track.uri,
        &track.title,
        &track.artist,
        &track.album,
        track.duration_ms,
        artwork_hash.as_deref(),
    )
}

/// Download artwork from a URL and cache it in the database.
async fn download_and_cache_artwork(db: &Database, url: &str) -> Result<String> {
    let response = reqwest::get(url).await?;
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    let bytes = response.bytes().await?;
    let hash = db.cache_artwork(&bytes, &content_type)?;
    Ok(hash)
}

#[derive(Debug, Default)]
pub struct SyncStats {
    pub tracks_synced: usize,
    pub playlists_synced: usize,
    pub errors: usize,
}
