use gpui::*;

use harmonia_core::models::{PlaybackState, UnifiedTrack};
use crate::theme::HarmoniaTheme;

/// Render the bottom transport / now-playing bar.
pub fn render_transport(
    current_track: Option<&UnifiedTrack>,
    state: PlaybackState,
    position_ms: u64,
    duration_ms: u64,
    volume: f32,
    theme: &HarmoniaTheme,
    on_prev: impl Fn() + 'static,
    on_play_pause: impl Fn() + 'static,
    on_next: impl Fn() + 'static,
) -> impl IntoElement {
    let progress_pct = if duration_ms > 0 {
        (position_ms as f32 / duration_ms as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };

    div()
        .flex()
        .items_center()
        .w_full()
        .h(px(80.0))
        .bg(theme.bg_secondary)
        .border_t_1()
        .border_color(theme.border)
        .px(px(16.0))
        .child(
            // Track info (left)
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .w(px(250.0))
                .child(
                    // Album art placeholder
                    div()
                        .size(px(56.0))
                        .rounded(px(4.0))
                        .bg(theme.bg_tertiary)
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(theme.text_primary)
                                .overflow_hidden()
                                .child(
                                    current_track
                                        .map(|t| t.display_title().to_string())
                                        .unwrap_or_else(|| "No track playing".to_string())
                                )
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_secondary)
                                .overflow_hidden()
                                .child(
                                    current_track
                                        .map(|t| t.display_artist().to_string())
                                        .unwrap_or_default()
                                )
                        )
                )
        )
        .child(
            // Transport controls (center)
            div()
                .flex()
                .flex_1()
                .flex_col()
                .items_center()
                .gap(px(4.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(16.0))
                        // Previous
                        .child(
                            div()
                                .id("prev-btn")
                                .cursor_pointer()
                                .text_size(px(18.0))
                                .text_color(theme.text_secondary)
                                .hover(|this| this.text_color(theme.text_primary))
                                .on_click(move |_, _, _cx| on_prev())
                                .child("⏮")
                        )
                        // Play/Pause
                        .child(
                            div()
                                .id("play-btn")
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(px(36.0))
                                .rounded_full()
                                .bg(theme.text_primary)
                                .hover(|this| this.bg(theme.accent))
                                .on_click(move |_, _, _cx| on_play_pause())
                                .child(
                                    div()
                                        .text_size(px(18.0))
                                        .text_color(theme.bg_primary)
                                        .child(if state == PlaybackState::Playing { "⏸" } else { "▶" })
                                )
                        )
                        // Next
                        .child(
                            div()
                                .id("next-btn")
                                .cursor_pointer()
                                .text_size(px(18.0))
                                .text_color(theme.text_secondary)
                                .hover(|this| this.text_color(theme.text_primary))
                                .on_click(move |_, _, _cx| on_next())
                                .child("⏭")
                        )
                )
                // Progress bar
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .w(px(400.0))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.text_muted)
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
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.text_muted)
                                .child(format_duration(duration_ms))
                        )
                )
        )
        .child(
            // Volume (right)
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .w(px(150.0))
                .justify_end()
                .child(
                    div()
                        .text_size(px(16.0))
                        .text_color(theme.text_secondary)
                        .child(if volume > 0.0 { "🔊" } else { "🔇" })
                )
                .child(
                    div()
                        .w(px(80.0))
                        .h(px(4.0))
                        .rounded(px(2.0))
                        .bg(theme.progress_bg)
                        .child(
                            div()
                                .h_full()
                                .rounded(px(2.0))
                                .bg(theme.text_secondary)
                                .w(relative(volume))
                        )
                )
        )
}

/// Format milliseconds as "m:ss".
fn format_duration(ms: u64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins}:{secs:02}")
}
