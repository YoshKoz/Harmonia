use gpui::*;

use harmonia_core::config::AppConfig;
use harmonia_core::db::Database;
use harmonia_core::scanner;
use harmonia_audio::router::AudioRouter;
use harmonia_spotify::auth::SpotifyAuth;
use harmonia_ui::app::{ActiveView, AppState};
use harmonia_ui::components::{sidebar, transport};
use harmonia_ui::views::{album_detail, album_grid, library, now_playing, playlist, playlist_detail, search, settings};

/// Root window view holding all application state.
struct Harmonia {
    state: AppState,
}

impl Render for Harmonia {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.state.theme.clone();
        let active_view = self.state.active_view;
        let entity = cx.entity().clone();

        // Read live playback state from audio router
        let playback_state = self.state.audio.lock().state();
        let position_ms = self.state.audio.lock().position_ms();
        let duration_ms = self.state.current_track.as_ref().map(|t| t.duration_ms).unwrap_or(0);
        let volume = self.state.audio.lock().volume();

        // Sidebar navigation callback
        let nav_entity = entity.clone();
        let on_navigate = move |view: ActiveView, _w: &mut Window, cx: &mut App| {
            nav_entity.update(cx, |this, cx| {
                this.state.active_view = view;
                cx.notify();
            });
        };

        // Transport callbacks
        let prev_entity = entity.clone();
        let on_prev = move |_w: &mut Window, cx: &mut App| {
            prev_entity.update(cx, |this, cx| {
                this.state.prev_track();
                cx.notify();
            });
        };

        let pp_entity = entity.clone();
        let on_play_pause = move |_w: &mut Window, cx: &mut App| {
            pp_entity.update(cx, |this, cx| {
                this.state.audio.lock().toggle_play_pause();
                cx.notify();
            });
        };

        let next_entity = entity.clone();
        let on_next = move |_w: &mut Window, cx: &mut App| {
            next_entity.update(cx, |this, cx| {
                this.state.next_track();
                cx.notify();
            });
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg_primary)
            .text_color(theme.text_primary)
            // Top area: sidebar + content
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    // Sidebar
                    .child(sidebar::render_sidebar(
                        active_view,
                        &theme,
                        on_navigate,
                    ))
                    // Main content
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(self.render_content(entity.clone()))
                    )
            )
            // Transport bar
            .child(transport::render_transport(
                self.state.current_track.as_ref(),
                playback_state,
                position_ms,
                duration_ms,
                volume,
                &theme,
                on_prev,
                on_play_pause,
                on_next,
            ))
    }
}

impl Harmonia {
    fn render_content(&self, entity: Entity<Self>) -> AnyElement {
        let theme = &self.state.theme;

        match self.state.active_view {
            ActiveView::Library | ActiveView::Artists => {
                let e = entity.clone();
                library::render_library_view(
                    &self.state.tracks_cache,
                    None,
                    theme,
                    move |idx, _w, cx| {
                        let e = e.clone();
                        e.update(cx, |this, cx| {
                            let tracks = this.state.tracks_cache.clone();
                            this.state.play_tracks(tracks, idx);
                            cx.notify();
                        });
                    },
                ).into_any_element()
            }
            ActiveView::Albums => {
                let e = entity.clone();
                album_grid::render_album_grid(
                    &self.state.albums_cache,
                    180,
                    theme,
                    move |idx, _w, cx| {
                        let e = e.clone();
                        e.update(cx, |this, cx| {
                            this.state.navigate_to_album(idx);
                            cx.notify();
                        });
                    },
                ).into_any_element()
            }
            ActiveView::AlbumDetail => {
                if let Some(album) = &self.state.selected_album {
                    let back_entity = entity.clone();
                    let play_entity = entity.clone();
                    album_detail::render_album_detail(
                        album,
                        &self.state.album_tracks_cache,
                        theme,
                        move |_w, cx| {
                            back_entity.update(cx, |this, cx| {
                                this.state.active_view = ActiveView::Albums;
                                cx.notify();
                            });
                        },
                        move |idx, _w, cx| {
                            let e = play_entity.clone();
                            e.update(cx, |this, cx| {
                                let tracks = this.state.album_tracks_cache.clone();
                                this.state.play_tracks(tracks, idx);
                                cx.notify();
                            });
                        },
                    ).into_any_element()
                } else {
                    div().into_any_element()
                }
            }
            ActiveView::Playlists => {
                let e = entity.clone();
                playlist::render_playlist_view(
                    &self.state.playlists_cache,
                    theme,
                    move |idx, _w, cx| {
                        let e = e.clone();
                        e.update(cx, |this, cx| {
                            this.state.navigate_to_playlist(idx);
                            cx.notify();
                        });
                    },
                ).into_any_element()
            }
            ActiveView::PlaylistDetail => {
                if let Some(playlist) = &self.state.selected_playlist {
                    let back_entity = entity.clone();
                    let play_entity = entity.clone();
                    playlist_detail::render_playlist_detail(
                        playlist,
                        &self.state.playlist_tracks_cache,
                        theme,
                        move |_w, cx| {
                            back_entity.update(cx, |this, cx| {
                                this.state.active_view = ActiveView::Playlists;
                                cx.notify();
                            });
                        },
                        move |idx, _w, cx| {
                            let e = play_entity.clone();
                            e.update(cx, |this, cx| {
                                let tracks = this.state.playlist_tracks_cache.clone();
                                this.state.play_tracks(tracks, idx);
                                cx.notify();
                            });
                        },
                    ).into_any_element()
                } else {
                    div().into_any_element()
                }
            }
            ActiveView::Search => {
                let search_entity = entity.clone();
                let play_entity = entity.clone();
                search::render_search_view(
                    &self.state.search_query,
                    &self.state.search_results,
                    theme,
                    move |_q, _w, _cx| {
                        // Search input not yet interactive — requires GPUI text input widget
                    },
                    move |idx, _w, cx| {
                        let e = play_entity.clone();
                        e.update(cx, |this, cx| {
                            let tracks = this.state.search_results.clone();
                            this.state.play_tracks(tracks, idx);
                            cx.notify();
                        });
                    },
                ).into_any_element()
            }
            ActiveView::NowPlaying => {
                let playback_state = self.state.audio.lock().state();
                let position_ms = self.state.audio.lock().position_ms();
                let duration_ms = self.state.current_track.as_ref().map(|t| t.duration_ms).unwrap_or(0);
                now_playing::render_now_playing(
                    self.state.current_track.as_ref(),
                    playback_state,
                    position_ms,
                    duration_ms,
                    theme,
                ).into_any_element()
            }
            ActiveView::Settings => {
                let library_dirs: Vec<String> = self.state.music_folders
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                settings::render_settings_view(
                    theme,
                    &library_dirs,
                    false,
                ).into_any_element()
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("harmonia=info".parse().unwrap()),
        )
        .init();

    tracing::info!("Starting Harmonia");

    let config = AppConfig::load().expect("Failed to load config");

    // ── --spotify-login: one-time credential setup, then exit ────────────────
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--spotify-login") {
        do_spotify_login();
        return;
    }

    let db = Database::open(&AppConfig::db_path()).expect("Failed to open database");
    let mut audio = AudioRouter::new().expect("Failed to create audio router");
    let selected_theme = config.ui.theme.clone();

    // Enable Spotify streaming — loads cached credentials automatically.
    // Run `harmonia --spotify-login` once to authenticate.
    let stream_cache = AppConfig::app_data_dir().join("spotify-stream");
    audio.enable_spotify(stream_cache);

    // Spotify library sync (metadata) via OAuth — runs in background, never blocks UI.
    // Requires spotify.client_id in config.toml (register at developer.spotify.com).
    if let Some(client_id) = config.spotify.client_id.clone() {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio rt");
            rt.block_on(async move {
                match SpotifyAuth::new(AppConfig::spotify_cache_dir(), &client_id) {
                    Ok(mut auth) => {
                        if let Err(e) = auth.authenticate().await {
                            tracing::warn!("Spotify OAuth failed: {e}");
                        }
                    }
                    Err(e) => tracing::warn!("Spotify auth init failed: {e}"),
                }
            });
        });
    } else {
        tracing::info!(
            "Spotify library sync disabled — set spotify.client_id in {}",
            AppConfig::config_path().display()
        );
    }

    let scan_on_startup = config.library.scan_on_startup;
    let music_folders = config.library.music_folders.clone();

    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                focus: true,
                show: true,
                titlebar: Some(TitlebarOptions {
                    title: Some("Harmonia".into()),
                    // Keep the entire window surface under app control so host chrome
                    // colors do not bleed through when the selected theme changes.
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_background: WindowBackgroundAppearance::Opaque,
                ..Default::default()
            },
            move |_window, cx| {
                if scan_on_startup {
                    let track_count = db.get_tracks(None).map(|t| t.len()).unwrap_or(0);
                    if track_count == 0 {
                        // First run: scan now so the library is populated immediately.
                        let _ = scanner::scan_library(&db, &music_folders, None);
                    } else {
                        // Subsequent runs: scan in background, don't block startup.
                        let bg_db = db.clone();
                        let bg_folders = music_folders.clone();
                        std::thread::spawn(move || {
                            let _ = scanner::scan_library(&bg_db, &bg_folders, None);
                        });
                    }
                }
                let mut state = AppState::new(db, audio, selected_theme, music_folders);
                state.refresh_library();
                cx.new(|_| Harmonia { state })
            },
        )
        .expect("Failed to open window");

        cx.activate(true);
    });
}

/// Interactive one-time Spotify login.
/// Prompts for username + password (password is hidden), authenticates via
/// librespot, caches an encrypted credential blob, then exits.
/// The password is NEVER written to disk.
fn do_spotify_login() {
    use harmonia_audio::spotify_player::SpotifyPlayer;
    use std::io::{self, Write};

    println!("── Harmonia Spotify Login ──────────────────────────────────");
    println!("Your password is used once to authenticate and is immediately");
    println!("discarded. An encrypted blob is cached for future launches.");
    println!();

    print!("Spotify username (email): ");
    io::stdout().flush().unwrap();
    let mut username = String::new();
    io::stdin().read_line(&mut username).unwrap();
    let username = username.trim().to_string();

    let password = rpassword::prompt_password("Spotify password: ")
        .expect("failed to read password");

    // SpotifyPlayer will connect, cache the encrypted blob, and confirm
    let cache_dir = AppConfig::app_data_dir().join("spotify-stream");
    let (tx, rx) = crossbeam_channel::bounded::<harmonia_audio::PlaybackEvent>(32);
    let player = SpotifyPlayer::new(cache_dir, tx);
    player.login(username, password);

    // Wait for success or failure (timeout after 30s)
    println!("Authenticating...");
    let timeout = std::time::Duration::from_secs(30);
    loop {
        match rx.recv_timeout(timeout) {
            Ok(harmonia_audio::PlaybackEvent::StateChanged(_)) => {
                println!("Spotify login successful. Launch Harmonia normally.");
                return;
            }
            Ok(harmonia_audio::PlaybackEvent::Error(e)) => {
                eprintln!("Login failed: {e}");
                return;
            }
            Ok(_) => continue,
            Err(_) => {
                eprintln!("Login timed out.");
                return;
            }
        }
    }
}
