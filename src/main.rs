use gpui::*;

use harmonia_core::config::AppConfig;
use harmonia_core::db::Database;
use harmonia_audio::router::AudioRouter;
use harmonia_ui::app::{ActiveView, AppState};
use harmonia_ui::theme::HarmoniaTheme;
use harmonia_ui::components::{sidebar, transport, track_list};
use harmonia_ui::views::{album_grid, library, now_playing, playlist, search};

/// Root window view holding all application state.
struct Harmonia {
    state: AppState,
}

impl Render for Harmonia {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.state.theme;

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
                        self.state.active_view,
                        theme,
                        {
                            let ptr = &mut self.state as *mut AppState;
                            move |view| unsafe { (*ptr).active_view = view; }
                        },
                    ))
                    // Main content
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(self.render_content())
                    )
            )
            // Transport bar
            .child(transport::render_transport(
                self.state.current_track.as_ref(),
                harmonia_core::models::PlaybackState::Stopped,
                0,
                self.state.current_track.as_ref().map(|t| t.duration_ms).unwrap_or(0),
                theme,
            ))
    }
}

impl Harmonia {
    fn render_content(&self) -> impl IntoElement {
        let theme = &self.state.theme;

        match self.state.active_view {
            ActiveView::Library | ActiveView::Artists => {
                library::render_library_view(
                    &self.state.tracks_cache,
                    None,
                    theme,
                    |_idx| { /* handled via cx in future */ },
                ).into_any_element()
            }
            ActiveView::Albums => {
                album_grid::render_album_grid(
                    &self.state.albums_cache,
                    180,
                    theme,
                    |_idx| {},
                ).into_any_element()
            }
            ActiveView::Playlists => {
                playlist::render_playlist_view(
                    &self.state.playlists_cache,
                    theme,
                    |_idx| {},
                ).into_any_element()
            }
            ActiveView::Search => {
                search::render_search_view(
                    &self.state.search_query,
                    &self.state.tracks_cache, // placeholder
                    theme,
                    |_q| {},
                    |_idx| {},
                ).into_any_element()
            }
            ActiveView::NowPlaying => {
                now_playing::render_now_playing(
                    self.state.current_track.as_ref(),
                    harmonia_core::models::PlaybackState::Stopped,
                    0,
                    self.state.current_track.as_ref().map(|t| t.duration_ms).unwrap_or(0),
                    theme,
                ).into_any_element()
            }
            ActiveView::Settings => {
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size_full()
                    .child(
                        div()
                            .text_size(px(18.0))
                            .text_color(theme.text_muted)
                            .child("Settings — coming soon")
                    )
                    .into_any_element()
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

    let config = AppConfig::load();
    let db = Database::open(&config.db_path()).expect("Failed to open database");
    let audio = AudioRouter::new();

    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                focus: true,
                show: true,
                titlebar: Some(TitlebarOptions {
                    title: Some("Harmonia".into()),
                    ..Default::default()
                }),
                window_background: WindowBackgroundAppearance::Opaque,
                ..Default::default()
            },
            |_window, cx| {
                let mut state = AppState::new(db, audio);
                state.refresh_library();
                cx.new(|_| Harmonia { state })
            },
        )
        .expect("Failed to open window");

        cx.activate(true);
    });
}
