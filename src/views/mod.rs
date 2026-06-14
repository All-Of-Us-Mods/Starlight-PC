pub mod explore;
pub mod home;
pub mod library;
pub mod library_detail;
pub mod mod_detail;
pub mod news_detail;
pub mod settings;

use crate::theme::Theme;
use gpui::*;

/// Muted, small-caps heading used above content sections in detail views.
pub fn section_label(text: &'static str, theme: &Theme) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.text_muted)
        .child(text)
}

/// Dimmed full-screen backdrop with a centered card. `children` are laid out
/// in the card as a vertical, gap-3 stack.
pub fn modal_overlay(
    theme: &Theme,
    width: Pixels,
    children: impl IntoIterator<Item = AnyElement>,
) -> Div {
    div()
        .absolute()
        .inset_0()
        .bg(Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.6,
        })
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .w(width)
                .p_5()
                .rounded_lg()
                .bg(theme.background)
                .border_1()
                .border_color(theme.border)
                .children(children),
        )
}
