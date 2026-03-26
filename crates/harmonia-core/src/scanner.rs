use std::path::{Path, PathBuf};
use anyhow::Result;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use tracing::{info, warn, debug};

use crate::db::Database;

/// Supported audio file extensions.
const AUDIO_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "ogg", "opus", "m4a", "aac", "wav", "aiff", "wma",
];

/// Progress info emitted during scanning.
#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub files_found: usize,
    pub files_scanned: usize,
    pub current_file: String,
    pub errors: usize,
}

/// Scan music folders and populate the database.
pub fn scan_library(db: &Database, folders: &[PathBuf], progress_tx: Option<&crossbeam_channel::Sender<ScanProgress>>) -> Result<ScanProgress> {
    let mut progress = ScanProgress {
        files_found: 0,
        files_scanned: 0,
        current_file: String::new(),
        errors: 0,
    };

    // Collect all audio files first
    let mut audio_files: Vec<PathBuf> = Vec::new();
    for folder in folders {
        if folder.is_dir() {
            collect_audio_files(folder, &mut audio_files);
        } else {
            warn!("Skipping non-directory: {}", folder.display());
        }
    }
    progress.files_found = audio_files.len();
    info!("Found {} audio files to scan", audio_files.len());

    // Process each file
    for file_path in &audio_files {
        progress.current_file = file_path.display().to_string();
        progress.files_scanned += 1;

        if let Some(tx) = progress_tx {
            let _ = tx.try_send(progress.clone());
        }

        if let Err(e) = process_audio_file(db, file_path) {
            debug!("Error scanning {}: {}", file_path.display(), e);
            progress.errors += 1;
        }
    }

    // Rebuild album index
    db.rebuild_albums()?;

    info!(
        "Scan complete: {} files processed, {} errors",
        progress.files_scanned, progress.errors
    );
    Ok(progress)
}

/// Recursively collect audio files from a directory.
fn collect_audio_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_audio_files(&path, files);
        } else if is_audio_file(&path) {
            files.push(path);
        }
    }
}

/// Check if a file is a supported audio format.
fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

/// Read metadata from an audio file and insert into the database.
fn process_audio_file(db: &Database, path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();

    let tagged_file = lofty::read_from_path(path)?;
    let tag = tagged_file.primary_tag()
        .or_else(|| tagged_file.first_tag());

    let (title, artist, album_artist, album, genre, year, track_num, disc_num) = if let Some(tag) = tag {
        (
            tag.title().unwrap_or_default().to_string(),
            tag.artist().unwrap_or_default().to_string(),
            tag.get_string(&lofty::tag::ItemKey::AlbumArtist)
                .map(|s| s.to_string())
                .unwrap_or_else(|| tag.artist().unwrap_or_default().to_string()),
            tag.album().unwrap_or_default().to_string(),
            tag.genre().unwrap_or_default().to_string(),
            tag.year(),
            tag.track(),
            tag.disk(),
        )
    } else {
        // No tags — use filename as title
        let title = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        (title, String::new(), String::new(), String::new(), String::new(), None, None, None)
    };

    // Extract duration from file properties
    let duration_ms = tagged_file.properties().duration().as_millis() as u64;

    // Extract and cache album art
    let artwork_hash = if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
        extract_and_cache_artwork(db, tag)?
    } else {
        None
    };

    db.upsert_local_track(
        &path_str,
        &title,
        &artist,
        &album_artist,
        &album,
        &genre,
        year.map(|y| y as i32),
        track_num,
        disc_num,
        duration_ms,
        artwork_hash.as_deref(),
    )?;

    Ok(())
}

/// Extract album artwork from tag and cache it in the database.
fn extract_and_cache_artwork(db: &Database, tag: &lofty::tag::Tag) -> Result<Option<String>> {
    use lofty::picture::PictureType;

    // Prefer front cover, fall back to any picture
    let picture = tag.get_picture_type(PictureType::CoverFront)
        .or_else(|| tag.pictures().first());

    if let Some(pic) = picture {
        let data = pic.data();
        if !data.is_empty() {
            let mime = pic.mime_type()
                .map(|m| m.to_string())
                .unwrap_or_else(|| "image/jpeg".to_string());
            let hash = db.cache_artwork(data, &mime)?;
            return Ok(Some(hash));
        }
    }

    Ok(None)
}
