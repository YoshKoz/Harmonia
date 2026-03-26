use gpui::*;
use gpui::prelude::FluentBuilder;

use harmonia_core::models::UnifiedTrack;
use crate::theme::HarmoniaTheme;
use crate::components::track_list::render_track_list;

/// Search view with input and results.
pub fn render_search_view(
    query: &str,
    results: &[UnifiedTrack],
    theme: &HarmoniaTheme,
    on_query_change: impl Fn(String) + 'static,
    on_track_click: impl Fn(usize) + 'static + Clone,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        // Search input area
        .child(
            div()
                .px(px(24.0))
                .py(px(16.0))
                .child(
                    div()
                        .text_size(px(24.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.text_primary)
                        .child("Search")
                )
                .child(
                    div()
                        .mt(px(12.0))
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(6.0))
                        .bg(theme.bg_tertiary)
                        .text_size(px(16.0))
                        .text_color(theme.text_primary)
                        .child(if query.is_empty() {
                            "Type to search...".to_string()
                        } else {
                            format!("🔍 {query}")
                        })
                )
        )
        // Results
        .when(!results.is_empty(), |this: gpui::Div| {
            this.child(
                div()
                    .px(px(24.0))
                    .py(px(8.0))
                    .text_size(px(14.0))
                    .text_color(theme.text_muted)
                    .child(format!("{} results", results.len()))
            )
            .child(render_track_list(results, None, theme, on_track_click))
        })
        .when(results.is_empty() && !query.is_empty(), |this: gpui::Div| {
            this.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    .child(
                        div()
                            .text_size(px(16.0))
                            .text_color(theme.text_muted)
                            .child("No results found")
                    )
            )
        })
}
