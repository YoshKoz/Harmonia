use gpui::*;
use gpui::prelude::FluentBuilder;

use harmonia_core::models::UnifiedTrack;
use crate::theme::HarmoniaTheme;

/// Render a scrollable track list with columns.
pub fn render_track_list(
    tracks: &[UnifiedTrack],
    selected_index: Option<usize>,
    theme: &HarmoniaTheme,
    on_track_click: impl Fn(usize, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .overflow_y_hidden()
        // Header row
        .child(
            div()
                .flex()
                .items_center()
                .px(px(16.0))
                .py(px(8.0))
                .border_b_1()
                .border_color(theme.border)
                .child(div().w(px(40.0)).text_size(px(12.0)).text_color(theme.text_muted).child("#"))
                .child(div().flex_1().text_size(px(12.0)).text_color(theme.text_muted).child("Title"))
                .child(div().w(px(200.0)).text_size(px(12.0)).text_color(theme.text_muted).child("Artist"))
                .child(div().w(px(200.0)).text_size(px(12.0)).text_color(theme.text_muted).child("Album"))
                .child(div().w(px(60.0)).text_size(px(12.0)).text_color(theme.text_muted).child("Duration"))
        )
        // Track rows
        .children(tracks.iter().enumerate().map(|(i, track)| {
            let is_selected = selected_index == Some(i);
            let on_click = on_track_click.clone();
            let track_num = track.track_number.unwrap_or((i + 1) as u32);

            div()
                .id(ElementId::Name(format!("track-{}", track.id).into()))
                .flex()
                .items_center()
                .px(px(16.0))
                .py(px(6.0))
                .cursor_pointer()
                .when(is_selected, |this: Stateful<gpui::Div>| this.bg(theme.selected))
                .hover(|style: StyleRefinement| style.bg(theme.hover))
                .on_click(move |_, window, cx| on_click(i, window, cx))
                // Track number
                .child(
                    div()
                        .w(px(40.0))
                        .text_size(px(14.0))
                        .text_color(theme.text_muted)
                        .child(track_num.to_string())
                )
                // Title + source indicator
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .overflow_hidden()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(theme.text_primary)
                                .overflow_hidden()
                                .child(track.display_title().to_string())
                        )
                        .when(track.is_spotify(), |this: gpui::Div| {
                            this.child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.accent)
                                    .child("●")
                            )
                        })
                )
                // Artist
                .child(
                    div()
                        .w(px(200.0))
                        .text_size(px(14.0))
                        .text_color(theme.text_secondary)
                        .overflow_hidden()
                        .child(track.display_artist().to_string())
                )
                // Album
                .child(
                    div()
                        .w(px(200.0))
                        .text_size(px(14.0))
                        .text_color(theme.text_secondary)
                        .overflow_hidden()
                        .child(track.album.clone())
                )
                // Duration
                .child(
                    div()
                        .w(px(60.0))
                        .text_size(px(14.0))
                        .text_color(theme.text_muted)
                        .child(format_duration(track.duration_ms))
                )
        }))
}

fn format_duration(ms: u64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins}:{secs:02}")
}
