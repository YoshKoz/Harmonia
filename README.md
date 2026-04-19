# Harmonia

A desktop music player for macOS and Linux that unifies your local library with Spotify streaming, built with Rust and [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) (the GPU-accelerated UI framework from Zed).

## Features

- **Local library** — scans FLAC, MP3, OGG, Opus, M4A, AAC, WAV, AIFF, WMA; reads embedded tags and album art
- **Spotify streaming** — stream any Spotify track directly (requires a Spotify account; no Premium required for library sync)
- **Unified library** — local files and Spotify tracks in one view
- **Albums, Artists, Playlists** — browse by view; click into album/playlist detail
- **SQLite library database** — fast startup after first scan; play count and rating tracking
- **Dark / Light theme** — set in `config.toml`
- **Gapless-ready** — per-track queue with prev/next controls

## Status

Early development. Core playback and library scanning work. Known gaps:

- Search input is read-only (GPUI text input integration pending)
- Settings view shows current library paths but changes require editing `config.toml` directly
- Windows support untested (GPUI targets macOS/Linux primarily)

## Build

Requires Rust 1.80+ and a C compiler.

```sh
git clone https://github.com/YoshKoz/Harmonia
cd Harmonia
cargo build --release
./target/release/harmonia
```

On Linux you also need ALSA dev headers:

```sh
# Debian/Ubuntu
sudo apt install libasound2-dev

# Arch
sudo pacman -S alsa-lib
```

## Configuration

On first launch, Harmonia creates `~/.config/Harmonia/config.toml` (Linux) or `~/Library/Application Support/Harmonia/config.toml` (macOS) with defaults:

```toml
[library]
music_folders = ["/home/you/Music"]
scan_on_startup = true
watch_for_changes = true

[spotify]
enabled = true
# client_id = "your-spotify-client-id"  # for library sync
sync_interval_minutes = 30

[audio]
gapless = true
crossfade_ms = 0
volume = 0.8

[ui]
theme = "Dark"  # or "Light"
album_grid_size = 180
```

## Spotify Setup

Harmonia uses two separate Spotify integrations:

### Streaming (play tracks)

Uses [librespot](https://github.com/librespot-org/librespot). Authenticate once:

```sh
harmonia --spotify-login
```

Enter your Spotify username and password. An encrypted credential blob is cached locally — your password is never written to disk.

### Library Sync (import playlists and saved tracks)

Register an app at [developer.spotify.com](https://developer.spotify.com/dashboard), set the redirect URI to `http://localhost:8888/callback`, then add your Client ID to `config.toml`:

```toml
[spotify]
client_id = "your-client-id"
```

Harmonia opens a browser for OAuth on first sync.

## How It Works

```
harmonia-core     — config, SQLite database, library scanner, data models
harmonia-audio    — local playback (symphonia + cpal) and Spotify streaming (librespot)
harmonia-spotify  — Spotify OAuth + library sync via rspotify
harmonia-ui       — GPUI views: sidebar, transport bar, library/album/playlist/search screens
```

Library scan runs on startup. First run scans synchronously so the library is ready when the window opens; subsequent runs scan in the background.

## Contributing

Issues and PRs welcome. The main areas that need work:

- Interactive search (GPUI text input)
- In-app settings editor
- File watcher for live library updates
- Shuffle / repeat modes
- Lyrics support (database schema is ready)

## License

MIT
