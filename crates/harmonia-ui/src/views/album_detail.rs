use gpui::*;

use harmonia_core::models::{Album, UnifiedTrack};
use crate::theme::HarmoniaTheme;
use crate::components::track_list::render_track_list;

/// Album detail view showing album info and its tracks.
pub fn render_album_detail(
    album: &Album,
    tracks: &[UnifiedTrack],
    theme: &HarmoniaTheme,
    on_back: impl Fn(&mut Window, &mut App) + 'static,
    on_track_click: impl Fn(usize, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        // Header with back button and album info
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(16.0))
                .px(px(24.0))
                .py(px(16.0))
                // Back button
                .child(
                    div()
                        .id("album-back")
                        .cursor_pointer()
                        .text_size(px(20.0))
                        .text_color(theme.text_secondary)
                        .hover(|style: StyleRefinement| style.text_color(theme.text_primary))
                        .on_click(move |_, window, cx| on_back(window, cx))
                        .child("←")
                )
                // Album artwork placeholder
                .child(
                    div()
                        .size(px(120.0))
                        .rounded(px(8.0))
                        .bg(theme.bg_tertiary)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_size(px(36.0))
                                .text_color(theme.text_muted)
                                .child("♫")
                        )
                )
                // Album info
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_muted)
                                .child("ALBUM")
                        )
                        .child(
                            div()
                                .text_size(px(24.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(theme.text_primary)
                                .child(album.title.clone())
                        )
                        .child(
                            div()
                                .text_size(px(16.0))
                                .text_color(theme.text_secondary)
                                .child(if album.artist.is_empty() {
                                    "Unknown Artist".to_string()
                                } else {
                                    album.artist.clone()
                                })
                        )
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(theme.text_muted)
                                .child(format!(
                                    "{} tracks{}",
                                    tracks.len(),
                                    album.year.map(|y| format!(" · {y}")).unwrap_or_default()
                                ))
                        )
                )
        )
        // Track list
        .child(render_track_list(tracks, None, theme, on_track_click))
}
