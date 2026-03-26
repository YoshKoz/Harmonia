use gpui::*;
use gpui::prelude::FluentBuilder;

use harmonia_core::models::Album;
use crate::theme::HarmoniaTheme;

/// Album grid view showing album artwork in a grid layout.
pub fn render_album_grid(
    albums: &[Album],
    grid_size: u32,
    theme: &HarmoniaTheme,
    on_album_click: impl Fn(usize) + 'static + Clone,
) -> impl IntoElement {
    let item_size = grid_size as f32;

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
                        .child("Albums")
                )
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(theme.text_muted)
                        .child(format!("{} albums", albums.len()))
                )
        )
        // Grid
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(16.0))
                .px(px(24.0))
                .pb(px(100.0)) // extra padding for bottom transport bar
                .children(albums.iter().enumerate().map(|(i, album)| {
                    let on_click = on_album_click.clone();

                    div()
                        .id(ElementId::Name(format!("album-{}", album.id).into()))
                        .flex()
                        .flex_col()
                        .w(px(item_size))
                        .cursor_pointer()
                        .rounded(px(8.0))
                        .overflow_hidden()
                        .hover(|style: StyleRefinement| style.bg(theme.hover))
                        .on_click(move |_, _, _cx| on_click(i))
                        // Artwork placeholder
                        .child(
                            div()
                                .w(px(item_size))
                                .h(px(item_size))
                                .rounded(px(4.0))
                                .bg(theme.bg_tertiary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .text_size(px(32.0))
                                        .text_color(theme.text_muted)
                                        .child("♫")
                                )
                        )
                        // Album title
                        .child(
                            div()
                                .px(px(4.0))
                                .pt(px(8.0))
                                .text_size(px(14.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.text_primary)
                                .overflow_hidden()
                                .child(album.title.clone())
                        )
                        // Artist
                        .child(
                            div()
                                .px(px(4.0))
                                .pt(px(2.0))
                                .pb(px(8.0))
                                .text_size(px(12.0))
                                .text_color(theme.text_secondary)
                                .overflow_hidden()
                                .child(if album.artist.is_empty() {
                                    "Unknown Artist".to_string()
                                } else {
                                    album.artist.clone()
                                })
                        )
                }))
        )
}
