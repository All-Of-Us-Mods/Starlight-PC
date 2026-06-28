pub mod explore;
pub mod home;
pub mod library;
pub mod library_detail;
pub mod mod_detail;
pub mod news_detail;
pub mod settings;

use crate::theme::Theme;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{Icon, IconName};

/// Shared outer container for every top-level page: full-size vertical flex
/// with the app font, base text color/size, and the standard page padding
/// (extra top padding clears the custom title bar). Callers chain on the bits
/// that vary per page — `.overflow_*()`, `.gap_*()`, `.relative()`, children.
pub fn page_root(id: &'static str, theme: &Theme) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .size_full()
        .font_family(crate::theme::FONT_FAMILY)
        .text_color(theme.text)
        .text_size(px(14.0))
        .p_8()
        .pt(px(48.0))
}

/// Ghost "Back" button with a left-arrow icon, shared by the detail views.
/// The caller injects the click handler, typically
/// `cx.listener(|_, _, _, cx| cx.emit(SomeDetailEvent::Close))`.
pub fn back_button(on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Button {
    Button::new("back")
        .ghost()
        .icon(Icon::new(IconName::ArrowLeft))
        .label("Back")
        .on_click(on_click)
}

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
