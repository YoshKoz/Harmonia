use gpui::*;

use harmonia_core::models::UnifiedTrack;
use crate::theme::HarmoniaTheme;
use crate::components::track_list::render_track_list;

/// Library view showing all tracks in the library.
pub fn render_library_view(
    tracks: &[UnifiedTrack],
    selected_index: Option<usize>,
    theme: &HarmoniaTheme,
    on_track_click: impl Fn(usize) + 'static + Clone,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        // Header
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(24.0))
                .py(px(16.0))
                .child(
                    div()
                        .text_size(px(24.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.text_primary)
                        .child("Library")
                )
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(theme.text_muted)
                        .child(format!("{} tracks", tracks.len()))
                )
        )
        // Track list
        .child(render_track_list(tracks, selected_index, theme, on_track_click))
}
