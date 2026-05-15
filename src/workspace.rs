use gpui::*;

use crate::theme::ThemeExt;
use crate::views::explore::ExploreView;
use crate::views::home::HomeView;
use crate::views::library::{LibraryEvent, LibraryView};
use crate::views::library_detail::LibraryDetailView;
use crate::views::settings::SettingsView;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Home,
    Explore,
    Library,
    Settings,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Explore => "Explore",
            Tab::Library => "Library",
            Tab::Settings => "Settings",
        }
    }
}

enum ActiveView {
    Home(Entity<HomeView>),
    Explore(Entity<ExploreView>),
    Library,
    LibraryDetail(Entity<LibraryDetailView>),
    Settings(Entity<SettingsView>),
}

pub struct Workspace {
    tab: Tab,
    view: ActiveView,
    library: Entity<LibraryView>,
    home: Entity<HomeView>,
    explore: Entity<ExploreView>,
    settings: Entity<SettingsView>,
}

impl Workspace {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let library = cx.new(|cx| LibraryView::new(cx));
        let home = cx.new(|cx| HomeView::new(cx));
        let explore = cx.new(|cx| ExploreView::new(cx));
        let settings = cx.new(|cx| SettingsView::new(cx));

        cx.subscribe(&library, |this, _, event: &LibraryEvent, cx| match event {
            LibraryEvent::Open(profile_id) => {
                let id = profile_id.clone();
                this.view = ActiveView::LibraryDetail(
                    cx.new(|cx| LibraryDetailView::new(id, cx)),
                );
                cx.notify();
            }
        })
        .detach();

        Self {
            tab: Tab::Library,
            view: ActiveView::Library,
            library,
            home,
            explore,
            settings,
        }
    }

    fn switch_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.tab = tab;
        self.view = match tab {
            Tab::Home => ActiveView::Home(self.home.clone()),
            Tab::Explore => ActiveView::Explore(self.explore.clone()),
            Tab::Library => ActiveView::Library,
            Tab::Settings => ActiveView::Settings(self.settings.clone()),
        };
        cx.notify();
    }

    fn nav_button(&self, tab: Tab, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let is_active = self.tab == tab;
        div()
            .id(SharedString::from(tab.label()))
            .px_4()
            .py_2()
            .rounded_md()
            .text_color(if is_active { theme.text } else { theme.text_muted })
            .bg(if is_active { theme.hover } else { rgba(0x00000000).into() })
            .hover(|s| s.bg(theme.hover))
            .cursor_pointer()
            .child(tab.label())
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.switch_tab(tab, cx);
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
                    .child(self.nav_button(Tab::Home, cx))
                    .child(self.nav_button(Tab::Explore, cx))
                    .child(self.nav_button(Tab::Library, cx))
                    .child(self.nav_button(Tab::Settings, cx)),
            )
    }

    fn render_content(&self) -> AnyElement {
        match &self.view {
            ActiveView::Home(v) => v.clone().into_any_element(),
            ActiveView::Explore(v) => v.clone().into_any_element(),
            ActiveView::Library => self.library.clone().into_any_element(),
            ActiveView::LibraryDetail(v) => v.clone().into_any_element(),
            ActiveView::Settings(v) => v.clone().into_any_element(),
        }
    }
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
