use gpui::*;

use crate::theme::ThemeExt;
use crate::views::library::LibraryView;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Home,
    Explore,
    Library,
    Settings,
}

impl View {
    fn label(self) -> &'static str {
        match self {
            View::Home => "Home",
            View::Explore => "Explore",
            View::Library => "Library",
            View::Settings => "Settings",
        }
    }
}

pub struct Workspace {
    current: View,
    library: Entity<LibraryView>,
}

impl Workspace {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let library = cx.new(|cx| LibraryView::new(cx));
        Self {
            current: View::Library,
            library,
        }
    }

    fn nav_button(&self, view: View, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let is_active = self.current == view;
        div()
            .id(SharedString::from(view.label()))
            .px_4()
            .py_2()
            .rounded_md()
            .text_color(if is_active { theme.text } else { theme.text_muted })
            .bg(if is_active { theme.hover } else { rgba(0x00000000).into() })
            .hover(|s| s.bg(theme.hover))
            .cursor_pointer()
            .child(view.label())
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.current = view;
                cx.notify();
            }))
    }

    fn render_sidenav(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .flex()
            .flex_col()
            .w(px(220.0))
            .h_full()
            .bg(theme.sidebar_background)
            .border_r_1()
            .border_color(theme.border)
            .pt(px(40.0))
            .px_3()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text)
                    .px_2()
                    .pb_4()
                    .child("Starlight"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(self.nav_button(View::Home, cx))
                    .child(self.nav_button(View::Explore, cx))
                    .child(self.nav_button(View::Library, cx))
                    .child(self.nav_button(View::Settings, cx)),
            )
    }

    fn render_content(&self) -> AnyElement {
        match self.current {
            View::Home => placeholder("Home").into_any_element(),
            View::Explore => placeholder("Explore").into_any_element(),
            View::Library => self.library.clone().into_any_element(),
            View::Settings => placeholder("Settings").into_any_element(),
        }
    }
}

fn placeholder(label: &'static str) -> impl IntoElement {
    div()
        .flex()
        .size_full()
        .items_center()
        .justify_center()
        .text_lg()
        .child(format!("{label} — not yet ported"))
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.text)
            .child(self.render_sidenav(cx))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .overflow_hidden()
                    .child(self.render_content()),
            )
    }
}
