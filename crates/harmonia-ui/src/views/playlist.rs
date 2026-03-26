use gpui::*;
use gpui::prelude::FluentBuilder;

use harmonia_core::models::Playlist;
use crate::theme::HarmoniaTheme;

/// Playlist list view.
pub fn render_playlist_view(
    playlists: &[Playlist],
    theme: &HarmoniaTheme,
    on_playlist_click: impl Fn(usize) + 'static + Clone,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .overflow_y_hidden()
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
                        .child("Playlists")
                )
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(theme.text_muted)
                        .child(format!("{} playlists", playlists.len()))
                )
        )
        // Playlist list
        .child(
            div()
                .flex()
                .flex_col()
                .px(px(8.0))
                .children(playlists.iter().enumerate().map(|(i, playlist)| {
                    let on_click = on_playlist_click.clone();
                    let source_badge = match &playlist.source {
                        harmonia_core::models::PlaylistSource::Local => "",
                        harmonia_core::models::PlaylistSource::Spotify(_) => "● Spotify",
                        harmonia_core::models::PlaylistSource::Mixed => "● Mixed",
                    };

                    div()
                        .id(ElementId::Name(format!("playlist-{}", playlist.id).into()))
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .px(px(16.0))
                        .py(px(10.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .hover(|style: StyleRefinement| style.bg(theme.hover))
                        .on_click(move |_, _, _cx| on_click(i))
                        // Icon
                        .child(
                            div()
                                .size(px(48.0))
                                .rounded(px(6.0))
                                .bg(theme.bg_tertiary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .text_size(px(20.0))
                                        .text_color(theme.text_muted)
                                        .child(if playlist.is_smart { "⚡" } else { "📋" })
                                )
                        )
                        // Info
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .text_size(px(15.0))
                                        .text_color(theme.text_primary)
                                        .child(playlist.name.clone())
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .text_size(px(13.0))
                                                .text_color(theme.text_secondary)
                                                .child(format!("{} tracks", playlist.track_count))
                                        )
                                        .when(!source_badge.is_empty(), |this: gpui::Div| {
                                            this.child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(theme.accent)
                                                    .child(source_badge)
                                            )
                                        })
                                )
                        )
                }))
        )
}
