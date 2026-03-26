use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration persisted to config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub library: LibraryConfig,
    pub spotify: SpotifyConfig,
    pub audio: AudioConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    pub music_folders: Vec<PathBuf>,
    pub scan_on_startup: bool,
    pub watch_for_changes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyConfig {
    pub enabled: bool,
    pub cache_path: Option<PathBuf>,
    pub sync_interval_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub output_device: Option<String>,
    pub gapless: bool,
    pub crossfade_ms: u32,
    pub volume: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: Theme,
    pub show_lyrics_panel: bool,
    pub album_grid_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
}

impl Default for AppConfig {
    fn default() -> Self {
        let music_dir = dirs::audio_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Music")))
            .unwrap_or_else(|| PathBuf::from("~/Music"));

        Self {
            library: LibraryConfig {
                music_folders: vec![music_dir],
                scan_on_startup: true,
                watch_for_changes: true,
            },
            spotify: SpotifyConfig {
                enabled: true,
                cache_path: None,
                sync_interval_minutes: 30,
            },
            audio: AudioConfig {
                output_device: None,
                gapless: true,
                crossfade_ms: 0,
                volume: 0.8,
            },
            ui: UiConfig {
                theme: Theme::Dark,
                show_lyrics_panel: false,
                album_grid_size: 180,
            },
        }
    }
}

impl AppConfig {
    /// Returns the app data directory (%APPDATA%/Harmonia).
    pub fn app_data_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Harmonia")
    }

    /// Path to the config file.
    pub fn config_path() -> PathBuf {
        Self::app_data_dir().join("config.toml")
    }

    /// Path to the library database.
    pub fn db_path() -> PathBuf {
        Self::app_data_dir().join("library.db")
    }

    /// Path to the Spotify credential cache.
    pub fn spotify_cache_dir() -> PathBuf {
        Self::app_data_dir().join("spotify-cache")
    }

    /// Load config from disk, creating default if it doesn't exist.
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Self = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save config to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
