/// Color theme for the application.
#[derive(Debug, Clone)]
pub struct HarmoniaTheme {
    // Background colors
    pub bg_primary: gpui::Rgba,
    pub bg_secondary: gpui::Rgba,
    pub bg_tertiary: gpui::Rgba,
    pub bg_elevated: gpui::Rgba,

    // Text colors
    pub text_primary: gpui::Rgba,
    pub text_secondary: gpui::Rgba,
    pub text_muted: gpui::Rgba,

    // Accent
    pub accent: gpui::Rgba,
    pub accent_hover: gpui::Rgba,

    // Borders
    pub border: gpui::Rgba,

    // States
    pub hover: gpui::Rgba,
    pub selected: gpui::Rgba,

    // Playback
    pub progress_bar: gpui::Rgba,
    pub progress_bg: gpui::Rgba,
}

impl HarmoniaTheme {
    pub fn dark() -> Self {
        Self {
            bg_primary: rgba(0x121212ff),
            bg_secondary: rgba(0x181818ff),
            bg_tertiary: rgba(0x282828ff),
            bg_elevated: rgba(0x333333ff),

            text_primary: rgba(0xffffffff),
            text_secondary: rgba(0xb3b3b3ff),
            text_muted: rgba(0x6a6a6aff),

            accent: rgba(0x1db954ff),       // Spotify green
            accent_hover: rgba(0x1ed760ff),

            border: rgba(0x333333ff),

            hover: rgba(0x2a2a2aff),
            selected: rgba(0x333333ff),

            progress_bar: rgba(0x1db954ff),
            progress_bg: rgba(0x4d4d4dff),
        }
    }

    pub fn light() -> Self {
        Self {
            bg_primary: rgba(0xf5f5f5ff),
            bg_secondary: rgba(0xffffffff),
            bg_tertiary: rgba(0xe8e8e8ff),
            bg_elevated: rgba(0xffffffff),

            text_primary: rgba(0x191414ff),
            text_secondary: rgba(0x535353ff),
            text_muted: rgba(0x999999ff),

            accent: rgba(0x1db954ff),
            accent_hover: rgba(0x1ed760ff),

            border: rgba(0xd9d9d9ff),

            hover: rgba(0xe8e8e8ff),
            selected: rgba(0xd9d9d9ff),

            progress_bar: rgba(0x1db954ff),
            progress_bg: rgba(0xd9d9d9ff),
        }
    }
}

/// Helper to convert a u32 RGBA hex to gpui::Rgba.
fn rgba(hex: u32) -> gpui::Rgba {
    gpui::rgba(hex)
}
