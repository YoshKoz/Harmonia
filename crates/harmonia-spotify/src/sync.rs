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
                if let Err(e) = upsert_spotify_track(db, track) {
                    warn!("Failed to sync track '{}': {e}", track.title);
                    stats.errors += 1;
                } else {
                    stats.tracks_synced += 1;
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
                                    if let Ok(_) = upsert_spotify_track(db, track) {
                                        // TODO: link track to playlist via playlist_tracks table
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

fn upsert_spotify_track(db: &Database, track: &SpotifyTrackInfo) -> Result<i64> {
    // TODO: download and cache artwork from track.image_url
    db.upsert_spotify_track(
        &track.uri,
        &track.title,
        &track.artist,
        &track.album,
        track.duration_ms,
        None, // artwork_hash - will be added when we download images
    )
}

#[derive(Debug, Default)]
pub struct SyncStats {
    pub tracks_synced: usize,
    pub playlists_synced: usize,
    pub errors: usize,
}
