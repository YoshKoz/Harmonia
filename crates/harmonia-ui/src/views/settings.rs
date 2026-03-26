use gpui::*;

use crate::theme::HarmoniaTheme;

/// Settings view displaying current configuration.
pub fn render_settings_view(
    theme: &HarmoniaTheme,
    library_dirs: &[String],
    spotify_connected: bool,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .overflow_y_hidden()
        // Header
        .child(
            div()
                .px(px(24.0))
                .py(px(16.0))
                .child(
                    div()
                        .text_size(px(24.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.text_primary)
                        .child("Settings")
                )
        )
        // Library section
        .child(render_section(
            "Library",
            theme,
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(render_setting_row(
                    "Music Folders",
                    if library_dirs.is_empty() {
                        "No folders configured".to_string()
                    } else {
                        library_dirs.join(", ")
                    },
                    theme,
                ))
        ))
        // Spotify section
        .child(render_section(
            "Spotify",
            theme,
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(render_setting_row(
                    "Connection",
                    if spotify_connected { "Connected" } else { "Not connected" }.to_string(),
                    theme,
                ))
        ))
        // Audio section
        .child(render_section(
            "Audio",
            theme,
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(render_setting_row("Output", "System Default".to_string(), theme))
                .child(render_setting_row("Gapless Playback", "Disabled".to_string(), theme))
        ))
        // About
        .child(render_section(
            "About",
            theme,
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(render_setting_row("Version", "0.1.0".to_string(), theme))
                .child(render_setting_row("Framework", "GPUI (Zed)".to_string(), theme))
        ))
}

fn render_section(
    title: &str,
    theme: &HarmoniaTheme,
    content: impl IntoElement,
) -> impl IntoElement {
    div()
        .px(px(24.0))
        .py(px(12.0))
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .pb(px(8.0))
                .border_b_1()
                .border_color(theme.border)
                .child(title.to_string())
        )
        .child(
            div()
                .pt(px(8.0))
                .child(content)
        )
}

fn render_setting_row(
    label: &str,
    value: String,
    theme: &HarmoniaTheme,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .py(px(4.0))
        .child(
            div()
                .text_size(px(14.0))
                .text_color(theme.text_secondary)
                .child(label.to_string())
        )
        .child(
            div()
                .text_size(px(14.0))
                .text_color(theme.text_primary)
                .child(value)
        )
}
