use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Arc;
use parking_lot::Mutex;
use tracing::info;

use crate::models::*;

/// Thread-safe database handle.
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open (or create) the library database.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.run_migrations()?;
        Ok(db)
    }

    /// Open an in-memory database (for tests).
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tracks (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                source          TEXT NOT NULL DEFAULT 'local',
                path            TEXT,
                spotify_uri     TEXT,
                title           TEXT NOT NULL DEFAULT '',
                artist          TEXT NOT NULL DEFAULT '',
                album_artist    TEXT NOT NULL DEFAULT '',
                album           TEXT NOT NULL DEFAULT '',
                genre           TEXT NOT NULL DEFAULT '',
                year            INTEGER,
                track_number    INTEGER,
                disc_number     INTEGER,
                duration_ms     INTEGER NOT NULL DEFAULT 0,
                artwork_hash    TEXT,
                rating          INTEGER,
                play_count      INTEGER NOT NULL DEFAULT 0,
                date_added      INTEGER NOT NULL,
                last_played     INTEGER,
                UNIQUE(path),
                UNIQUE(spotify_uri)
            );

            CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);
            CREATE INDEX IF NOT EXISTS idx_tracks_album_artist ON tracks(album_artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_source ON tracks(source);

            CREATE TABLE IF NOT EXISTS albums (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                title           TEXT NOT NULL,
                artist          TEXT NOT NULL DEFAULT '',
                year            INTEGER,
                artwork_hash    TEXT,
                track_count     INTEGER NOT NULL DEFAULT 0,
                UNIQUE(title, artist)
            );

            CREATE TABLE IF NOT EXISTS playlists (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                name            TEXT NOT NULL,
                source          TEXT NOT NULL DEFAULT 'local',
                spotify_id      TEXT,
                is_smart        INTEGER NOT NULL DEFAULT 0,
                smart_query     TEXT
            );

            CREATE TABLE IF NOT EXISTS playlist_tracks (
                playlist_id     INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
                track_id        INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
                position        INTEGER NOT NULL,
                PRIMARY KEY (playlist_id, track_id, position)
            );

            CREATE TABLE IF NOT EXISTS artwork_cache (
                hash            TEXT PRIMARY KEY,
                data            BLOB NOT NULL,
                mime            TEXT NOT NULL DEFAULT 'image/jpeg'
            );

            CREATE TABLE IF NOT EXISTS lyrics_cache (
                track_id        INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
                synced          INTEGER NOT NULL DEFAULT 0,
                content         TEXT NOT NULL
            );"
        )?;

        info!("Database migrations applied");
        Ok(())
    }

    // ─── Track Operations ────────────────────────────────────────────

    /// Insert or update a local track from scanned metadata.
    pub fn upsert_local_track(
        &self,
        path: &str,
        title: &str,
        artist: &str,
        album_artist: &str,
        album: &str,
        genre: &str,
        year: Option<i32>,
        track_number: Option<u32>,
        disc_number: Option<u32>,
        duration_ms: u64,
        artwork_hash: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO tracks (source, path, title, artist, album_artist, album, genre,
                                 year, track_number, disc_number, duration_ms, artwork_hash, date_added)
             VALUES ('local', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(path) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                album_artist = excluded.album_artist,
                album = excluded.album,
                genre = excluded.genre,
                year = excluded.year,
                track_number = excluded.track_number,
                disc_number = excluded.disc_number,
                duration_ms = excluded.duration_ms,
                artwork_hash = excluded.artwork_hash",
            params![path, title, artist, album_artist, album, genre,
                    year, track_number, disc_number, duration_ms as i64, artwork_hash, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Insert or update a Spotify track.
    pub fn upsert_spotify_track(
        &self,
        spotify_uri: &str,
        title: &str,
        artist: &str,
        album: &str,
        duration_ms: u64,
        artwork_hash: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO tracks (source, spotify_uri, title, artist, album_artist, album,
                                 duration_ms, artwork_hash, date_added)
             VALUES ('spotify', ?1, ?2, ?3, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(spotify_uri) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                album = excluded.album,
                duration_ms = excluded.duration_ms,
                artwork_hash = excluded.artwork_hash",
            params![spotify_uri, title, artist, album, duration_ms as i64, artwork_hash, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get all tracks, optionally filtered by source.
    pub fn get_tracks(&self, source_filter: Option<&str>) -> Result<Vec<UnifiedTrack>> {
        let conn = self.conn.lock();
        let mut stmt = if let Some(source) = source_filter {
            let mut s = conn.prepare(
                "SELECT id, source, path, spotify_uri, title, artist, album_artist, album,
                        genre, year, track_number, disc_number, duration_ms, artwork_hash,
                        rating, play_count, date_added, last_played
                 FROM tracks WHERE source = ?1 ORDER BY album_artist, album, disc_number, track_number"
            )?;
            let rows = s.query_map(params![source], row_to_track)?;
            return rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into);
        } else {
            conn.prepare(
                "SELECT id, source, path, spotify_uri, title, artist, album_artist, album,
                        genre, year, track_number, disc_number, duration_ms, artwork_hash,
                        rating, play_count, date_added, last_played
                 FROM tracks ORDER BY album_artist, album, disc_number, track_number"
            )?
        };
        let rows = stmt.query_map([], row_to_track)?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Search tracks by title/artist/album.
    pub fn search_tracks(&self, query: &str) -> Result<Vec<UnifiedTrack>> {
        let conn = self.conn.lock();
        let pattern = format!("%{query}%");
        let mut stmt = conn.prepare(
            "SELECT id, source, path, spotify_uri, title, artist, album_artist, album,
                    genre, year, track_number, disc_number, duration_ms, artwork_hash,
                    rating, play_count, date_added, last_played
             FROM tracks
             WHERE title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1
             ORDER BY title LIMIT 200"
        )?;
        let rows = stmt.query_map(params![pattern], row_to_track)?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get a single track by ID.
    pub fn get_track(&self, id: i64) -> Result<Option<UnifiedTrack>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, source, path, spotify_uri, title, artist, album_artist, album,
                    genre, year, track_number, disc_number, duration_ms, artwork_hash,
                    rating, play_count, date_added, last_played
             FROM tracks WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], row_to_track)?;
        match rows.next() {
            Some(Ok(track)) => Ok(Some(track)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    // ─── Album Operations ────────────────────────────────────────────

    /// Refresh the albums table from tracks data.
    pub fn rebuild_albums(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            "DELETE FROM albums;
             INSERT INTO albums (title, artist, year, artwork_hash, track_count)
             SELECT album, album_artist, MAX(year), MAX(artwork_hash), COUNT(*)
             FROM tracks
             WHERE album != ''
             GROUP BY album, album_artist;"
        )?;
        Ok(())
    }

    /// Get all albums.
    pub fn get_albums(&self) -> Result<Vec<Album>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, year, artwork_hash, track_count
             FROM albums ORDER BY artist, year, title"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Album {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                year: row.get(3)?,
                artwork_hash: row.get(4)?,
                track_count: row.get::<_, i64>(5)? as u32,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get tracks for a specific album.
    pub fn get_album_tracks(&self, album: &str, album_artist: &str) -> Result<Vec<UnifiedTrack>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, source, path, spotify_uri, title, artist, album_artist, album,
                    genre, year, track_number, disc_number, duration_ms, artwork_hash,
                    rating, play_count, date_added, last_played
             FROM tracks WHERE album = ?1 AND album_artist = ?2
             ORDER BY disc_number, track_number"
        )?;
        let rows = stmt.query_map(params![album, album_artist], row_to_track)?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    // ─── Playlist Operations ─────────────────────────────────────────

    /// Create a new playlist.
    pub fn create_playlist(&self, name: &str, source: &str) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO playlists (name, source) VALUES (?1, ?2)",
            params![name, source],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get all playlists.
    pub fn get_playlists(&self) -> Result<Vec<Playlist>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.source, p.spotify_id, p.is_smart, p.smart_query,
                    (SELECT COUNT(*) FROM playlist_tracks pt WHERE pt.playlist_id = p.id) as track_count
             FROM playlists p ORDER BY p.name"
        )?;
        let rows = stmt.query_map([], |row| {
            let source_str: String = row.get(2)?;
            let spotify_id: Option<String> = row.get(3)?;
            let source = match source_str.as_str() {
                "spotify" => PlaylistSource::Spotify(spotify_id.unwrap_or_default()),
                "mixed" => PlaylistSource::Mixed,
                _ => PlaylistSource::Local,
            };
            let smart_query_str: Option<String> = row.get(5)?;
            let smart_query = smart_query_str
                .and_then(|s| serde_json::from_str(&s).ok());
            Ok(Playlist {
                id: row.get(0)?,
                name: row.get(1)?,
                source,
                is_smart: row.get::<_, bool>(4)?,
                smart_query,
                track_count: row.get::<_, i64>(6)? as u32,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Add a track to a playlist at the end.
    pub fn add_track_to_playlist(&self, playlist_id: i64, track_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let next_pos: i64 = conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?1",
            params![playlist_id],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
             VALUES (?1, ?2, ?3)",
            params![playlist_id, track_id, next_pos],
        )?;
        Ok(())
    }

    /// Get tracks in a playlist ordered by position.
    pub fn get_playlist_tracks(&self, playlist_id: i64) -> Result<Vec<UnifiedTrack>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.source, t.path, t.spotify_uri, t.title, t.artist, t.album_artist,
                    t.album, t.genre, t.year, t.track_number, t.disc_number, t.duration_ms,
                    t.artwork_hash, t.rating, t.play_count, t.date_added, t.last_played
             FROM playlist_tracks pt
             JOIN tracks t ON t.id = pt.track_id
             WHERE pt.playlist_id = ?1
             ORDER BY pt.position"
        )?;
        let rows = stmt.query_map(params![playlist_id], row_to_track)?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    // ─── Artwork Cache ───────────────────────────────────────────────

    /// Store artwork in the cache, returning the hash.
    pub fn cache_artwork(&self, data: &[u8], mime: &str) -> Result<String> {
        use sha2::{Sha256, Digest};
        let hash = hex::encode(Sha256::digest(data));
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO artwork_cache (hash, data, mime) VALUES (?1, ?2, ?3)",
            params![hash, data, mime],
        )?;
        Ok(hash)
    }

    /// Get artwork by hash.
    pub fn get_artwork(&self, hash: &str) -> Result<Option<ArtworkEntry>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT hash, data, mime FROM artwork_cache WHERE hash = ?1"
        )?;
        let mut rows = stmt.query_map(params![hash], |row| {
            Ok(ArtworkEntry {
                hash: row.get(0)?,
                data: row.get(1)?,
                mime: row.get(2)?,
            })
        })?;
        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    // ─── Stats / Playback Tracking ───────────────────────────────────

    /// Increment play count and set last_played for a track.
    pub fn record_play(&self, track_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        conn.execute(
            "UPDATE tracks SET play_count = play_count + 1, last_played = ?1 WHERE id = ?2",
            params![now, track_id],
        )?;
        Ok(())
    }

    /// Set rating for a track (0-5).
    pub fn set_rating(&self, track_id: i64, rating: u8) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE tracks SET rating = ?1 WHERE id = ?2",
            params![rating.min(5), track_id],
        )?;
        Ok(())
    }
}

/// Helper: map a row to a UnifiedTrack.
fn row_to_track(row: &rusqlite::Row) -> rusqlite::Result<UnifiedTrack> {
    let source_str: String = row.get(1)?;
    let path: Option<String> = row.get(2)?;
    let spotify_uri: Option<String> = row.get(3)?;

    let source = match source_str.as_str() {
        "spotify" => {
            TrackSource::Spotify(spotify_uri.unwrap_or_default())
        }
        _ => {
            TrackSource::Local(path.unwrap_or_default().into())
        }
    };

    Ok(UnifiedTrack {
        id: row.get(0)?,
        source,
        title: row.get(4)?,
        artist: row.get(5)?,
        album_artist: row.get(6)?,
        album: row.get(7)?,
        genre: row.get(8)?,
        year: row.get(9)?,
        track_number: row.get::<_, Option<i64>>(10)?.map(|v| v as u32),
        disc_number: row.get::<_, Option<i64>>(11)?.map(|v| v as u32),
        duration_ms: row.get::<_, i64>(12)? as u64,
        artwork_hash: row.get(13)?,
        rating: row.get::<_, Option<i64>>(14)?.map(|v| v as u8),
        play_count: row.get::<_, i64>(15)? as u32,
        date_added: row.get(16)?,
        last_played: row.get(17)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_query() {
        let db = Database::open_memory().unwrap();
        let id = db.upsert_local_track(
            "/music/test.flac", "Test Song", "Test Artist", "Test Artist",
            "Test Album", "Rock", Some(2024), Some(1), Some(1), 240_000, None,
        ).unwrap();
        assert!(id > 0);

        let tracks = db.get_tracks(None).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "Test Song");
    }

    #[test]
    fn test_search() {
        let db = Database::open_memory().unwrap();
        db.upsert_local_track(
            "/a.flac", "Hello World", "Artist A", "Artist A",
            "Album X", "", None, None, None, 100_000, None,
        ).unwrap();
        db.upsert_local_track(
            "/b.flac", "Goodbye", "Artist B", "Artist B",
            "Album Y", "", None, None, None, 200_000, None,
        ).unwrap();

        let results = db.search_tracks("hello").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Hello World");
    }

    #[test]
    fn test_artwork_cache() {
        let db = Database::open_memory().unwrap();
        let data = b"fake image data";
        let hash = db.cache_artwork(data, "image/jpeg").unwrap();
        let entry = db.get_artwork(&hash).unwrap().unwrap();
        assert_eq!(entry.data, data);
        assert_eq!(entry.mime, "image/jpeg");
    }
}
