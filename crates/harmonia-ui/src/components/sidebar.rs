use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::app::ActiveView;
use crate::theme::HarmoniaTheme;

/// Sidebar navigation item.
struct NavItem {
    label: &'static str,
    icon: &'static str,
    view: ActiveView,
}

const NAV_ITEMS: &[NavItem] = &[
    NavItem {
        label: "Library",
        icon: "♫",
        view: ActiveView::Library,
    },
    NavItem {
        label: "Albums",
        icon: "💿",
        view: ActiveView::Albums,
    },
    NavItem {
        label: "Artists",
        icon: "🎤",
        view: ActiveView::Artists,
    },
    NavItem {
        label: "Playlists",
        icon: "📋",
        view: ActiveView::Playlists,
    },
    NavItem {
        label: "Search",
        icon: "🔍",
        view: ActiveView::Search,
    },
    NavItem {
        label: "Settings",
        icon: "⚙",
        view: ActiveView::Settings,
    },
];

/// Render the sidebar navigation.
pub fn render_sidebar(
    active: ActiveView,
    theme: &HarmoniaTheme,
    on_navigate: impl Fn(ActiveView, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .w(px(220.0))
        .h_full()
        .bg(theme.bg_secondary)
        .border_r_1()
        .border_color(theme.border)
        .child(
            // Logo / app name
            div().px(px(16.0)).py(px(20.0)).child(
                div()
                    .text_size(px(20.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child("Harmonia"),
            ),
        )
        .child(
            // Navigation items
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .px(px(8.0))
                .children(NAV_ITEMS.iter().map(|item| {
                    let is_active = item.view == active;
                    let view = item.view;
                    let on_nav = on_navigate.clone();

                    div()
                        .id(SharedString::from(item.label))
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .when(is_active, |this: Stateful<gpui::Div>| {
                            this.bg(theme.selected)
                        })
                        .hover(|style: StyleRefinement| style.bg(theme.hover))
                        .on_click(move |_, window, cx| {
                            on_nav(view, window, cx);
                        })
                        .child(div().text_size(px(16.0)).child(item.icon))
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(if is_active {
                                    theme.text_primary
                                } else {
                                    theme.text_secondary
                                })
                                .child(item.label),
                        )
                })),
        )
}
