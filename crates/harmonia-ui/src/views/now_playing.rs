use gpui::*;

use harmonia_core::models::{PlaybackState, UnifiedTrack};
use crate::theme::HarmoniaTheme;

/// Expanded now-playing view with large artwork and details.
pub fn render_now_playing(
    track: Option<&UnifiedTrack>,
    state: PlaybackState,
    position_ms: u64,
    duration_ms: u64,
    theme: &HarmoniaTheme,
) -> impl IntoElement {
    let progress_pct = if duration_ms > 0 {
        (position_ms as f32 / duration_ms as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };

    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .size_full()
        .gap(px(24.0))
        // Large artwork
        .child(
            div()
                .size(px(320.0))
                .rounded(px(12.0))
                .bg(theme.bg_tertiary)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_size(px(64.0))
                        .text_color(theme.text_muted)
                        .child("♫")
                )
        )
        // Track info
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(px(22.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.text_primary)
                        .child(
                            track.map(|t| t.display_title().to_string())
                                .unwrap_or_else(|| "No track playing".to_string())
                        )
                )
                .child(
                    div()
                        .text_size(px(16.0))
                        .text_color(theme.text_secondary)
                        .child(
                            track.map(|t| t.display_artist().to_string())
                                .unwrap_or_default()
                        )
                )
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(theme.text_muted)
                        .child(
                            track.map(|t| t.album.clone())
                                .unwrap_or_default()
                        )
                )
        )
        // Progress
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .w(px(400.0))
                .child(
                    div().text_size(px(12.0)).text_color(theme.text_muted)
                        .child(format_duration(position_ms))
                )
                .child(
                    div()
                        .flex_1()
                        .h(px(4.0))
                        .rounded(px(2.0))
                        .bg(theme.progress_bg)
                        .child(
                            div()
                                .h_full()
                                .rounded(px(2.0))
                                .bg(theme.progress_bar)
                                .w(relative(progress_pct))
                        )
                )
                .child(
                    div().text_size(px(12.0)).text_color(theme.text_muted)
                        .child(format_duration(duration_ms))
                )
        )
}

fn format_duration(ms: u64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins}:{secs:02}")
}
