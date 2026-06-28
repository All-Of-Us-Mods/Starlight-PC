use gpui::*;

use crate::theme::{self, ThemeExt};
use crate::ui::icon::AppIcon;
use crate::views::explore::ExploreView;
use crate::views::home::HomeView;
use crate::views::library::{LibraryEvent, LibraryView};
use crate::views::library_detail::{LibraryDetailEvent, LibraryDetailView};
use crate::views::mod_detail::ModDetailView;
use crate::views::news_detail::NewsDetailView;
use crate::views::settings::SettingsView;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::sidebar::{Sidebar, SidebarHeader, SidebarMenu, SidebarMenuItem};
use gpui_component::{Disableable, Icon, IconName, TitleBar};

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

    fn icon(self) -> Icon {
        match self {
            Tab::Home => Icon::new(AppIcon::Home),
            Tab::Explore => Icon::new(AppIcon::Compass),
            Tab::Library => Icon::new(AppIcon::Library),
            Tab::Settings => Icon::new(IconName::Settings),
        }
    }
}

/// One entry in the navigation history. Top-level tabs reuse the persistent
/// view entities held by [`Workspace`]; detail pages carry their own (fresh,
/// state-preserving) entity so back/forward can revisit them.
#[derive(Clone)]
enum Page {
    Home,
    Explore,
    Library,
    Settings,
    ModDetail(Entity<ModDetailView>),
    LibraryDetail(Entity<LibraryDetailView>),
    NewsDetail(Entity<NewsDetailView>),
}

pub struct Workspace {
    /// Browser-style navigation history; `cursor` is the entry currently shown.
    history: Vec<Page>,
    cursor: usize,
    library: Entity<LibraryView>,
    home: Entity<HomeView>,
    explore: Entity<ExploreView>,
    settings: Entity<SettingsView>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let library = cx.new(|cx| LibraryView::new(window, cx));
        let home = cx.new(HomeView::new);
        let explore = cx.new(|cx| ExploreView::new(window, cx));
        let settings = cx.new(SettingsView::new);

        cx.subscribe(
            &home,
            |this, _, ev: &crate::views::home::HomeEvent, cx| match ev {
                crate::views::home::HomeEvent::OpenMod(id) => this.open_mod(id.clone(), cx),
                crate::views::home::HomeEvent::OpenNews(post) => this.open_news(post.clone(), cx),
            },
        )
        .detach();
        cx.subscribe(
            &explore,
            |this, _, ev: &crate::views::explore::ExploreEvent, cx| match ev {
                crate::views::explore::ExploreEvent::OpenMod(id) => this.open_mod(id.clone(), cx),
            },
        )
        .detach();

        cx.subscribe_in(
            &library,
            window,
            |this, _, event: &LibraryEvent, window, cx| match event {
                LibraryEvent::Open(profile_id) => this.open_profile(profile_id.clone(), window, cx),
            },
        )
        .detach();

        Self {
            history: vec![Page::Library],
            cursor: 0,
            library,
            home,
            explore,
            settings,
        }
    }

    fn current(&self) -> &Page {
        &self.history[self.cursor]
    }

    fn can_go_back(&self) -> bool {
        self.cursor > 0
    }

    fn can_go_forward(&self) -> bool {
        self.cursor + 1 < self.history.len()
    }

    /// Push a new page, dropping any forward history (browser semantics).
    fn navigate(&mut self, page: Page, cx: &mut Context<Self>) {
        self.history.truncate(self.cursor + 1);
        self.history.push(page);
        self.cursor = self.history.len() - 1;
        self.after_navigate(cx);
    }

    fn go_back(&mut self, cx: &mut Context<Self>) {
        if self.can_go_back() {
            self.cursor -= 1;
            self.after_navigate(cx);
        }
    }

    fn go_forward(&mut self, cx: &mut Context<Self>) {
        if self.can_go_forward() {
            self.cursor += 1;
            self.after_navigate(cx);
        }
    }

    /// Drop the current page (and any forward history) and step back to the
    /// previous one. Used when a page closes itself, e.g. after a profile is
    /// deleted — it shouldn't linger in forward history.
    fn close_current(&mut self, cx: &mut Context<Self>) {
        if self.cursor > 0 {
            self.history.truncate(self.cursor);
            self.cursor -= 1;
        }
        self.after_navigate(cx);
    }

    /// Side effects whenever the current page changes. Profiles can change from
    /// anywhere (mod install, BepInEx install, …); pull a fresh list whenever
    /// the Library list becomes visible.
    fn after_navigate(&mut self, cx: &mut Context<Self>) {
        if matches!(self.current(), Page::Library) {
            self.library.update(cx, |lib, cx| lib.refresh(cx));
        }
        cx.notify();
    }

    /// The sidebar tab to highlight: the nearest top-level section at or behind
    /// the current page (so a mod detail opened from Explore keeps Explore lit).
    fn active_tab(&self) -> Option<Tab> {
        self.history[..=self.cursor]
            .iter()
            .rev()
            .find_map(|p| match p {
                Page::Home => Some(Tab::Home),
                Page::Explore => Some(Tab::Explore),
                Page::Library | Page::LibraryDetail(_) => Some(Tab::Library),
                Page::Settings => Some(Tab::Settings),
                Page::ModDetail(_) | Page::NewsDetail(_) => None,
            })
    }

    fn current_title(&self, cx: &App) -> SharedString {
        match self.current() {
            Page::Home => "Home".into(),
            Page::Explore => "Explore".into(),
            Page::Library => "Library".into(),
            Page::Settings => "Settings".into(),
            Page::ModDetail(v) => v.read(cx).title(),
            Page::LibraryDetail(v) => v.read(cx).title(),
            Page::NewsDetail(v) => v.read(cx).title(),
        }
    }

    fn switch_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        let already_here = matches!(
            (self.current(), tab),
            (Page::Home, Tab::Home)
                | (Page::Explore, Tab::Explore)
                | (Page::Library, Tab::Library)
                | (Page::Settings, Tab::Settings)
        );
        if already_here {
            return;
        }
        let page = match tab {
            Tab::Home => Page::Home,
            Tab::Explore => Page::Explore,
            Tab::Library => Page::Library,
            Tab::Settings => Page::Settings,
        };
        self.navigate(page, cx);
    }

    fn menu_item(&self, tab: Tab, cx: &mut Context<Self>) -> SidebarMenuItem {
        SidebarMenuItem::new(tab.label())
            .icon(tab.icon())
            .active(self.active_tab() == Some(tab))
            .on_click(cx.listener(move |this, _, _window, cx| this.switch_tab(tab, cx)))
    }

    fn render_sidenav(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Sidebar::new("starlight-sidebar")
            .collapsible(false)
            .header(
                SidebarHeader::new()
                    .child(
                        Icon::new(AppIcon::Starlight)
                            .size(px(28.0))
                            .text_color(rgb(0xffc107)),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .child("Starlight"),
                    ),
            )
            .child(
                SidebarMenu::new()
                    .child(self.menu_item(Tab::Home, cx))
                    .child(self.menu_item(Tab::Explore, cx))
                    .child(self.menu_item(Tab::Library, cx))
                    .child(self.menu_item(Tab::Settings, cx)),
            )
    }

    /// Back/forward navigation controls plus the current page title, shown in
    /// the custom title bar (replacing the per-page back buttons).
    fn render_nav(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(
                // The title bar treats a left mouse-down as the start of a window
                // drag; stop propagation here so clicks reach the nav buttons.
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        Button::new("nav-back")
                            .ghost()
                            .icon(Icon::new(IconName::ArrowLeft))
                            .disabled(!self.can_go_back())
                            .on_click(cx.listener(|this, _, _window, cx| this.go_back(cx))),
                    )
                    .child(
                        Button::new("nav-forward")
                            .ghost()
                            .icon(Icon::new(IconName::ArrowRight))
                            .disabled(!self.can_go_forward())
                            .on_click(cx.listener(|this, _, _window, cx| this.go_forward(cx))),
                    ),
            )
            .child(
                div()
                    .ml_1()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.current_title(cx)),
            )
    }

    fn render_content(&self) -> AnyElement {
        match self.current() {
            Page::Home => self.home.clone().into_any_element(),
            Page::Explore => self.explore.clone().into_any_element(),
            Page::Library => self.library.clone().into_any_element(),
            Page::Settings => self.settings.clone().into_any_element(),
            Page::ModDetail(v) => v.clone().into_any_element(),
            Page::LibraryDetail(v) => v.clone().into_any_element(),
            Page::NewsDetail(v) => v.clone().into_any_element(),
        }
    }

    fn open_mod(&mut self, mod_id: String, cx: &mut Context<Self>) {
        let detail = cx.new(|cx| ModDetailView::new(mod_id, cx));
        // Re-render the title bar when the mod finishes loading (title updates).
        cx.observe(&detail, |_, _, cx| cx.notify()).detach();
        self.navigate(Page::ModDetail(detail), cx);
    }

    fn open_news(&mut self, post: crate::backend::api::Post, cx: &mut Context<Self>) {
        let detail = cx.new(|_| NewsDetailView::new(post));
        self.navigate(Page::NewsDetail(detail), cx);
    }

    fn open_profile(&mut self, profile_id: String, window: &mut Window, cx: &mut Context<Self>) {
        let detail = cx.new(|cx| LibraryDetailView::new(profile_id, window, cx));
        // Keep the title bar in sync once the profile loads (title updates).
        cx.observe(&detail, |_, _, cx| cx.notify()).detach();
        cx.subscribe(&detail, |this, _, ev: &LibraryDetailEvent, cx| match ev {
            // The detail view closes itself after deleting its profile; drop it
            // from history rather than leaving a dangling "not found" page.
            LibraryDetailEvent::Close => this.close_current(cx),
        })
        .detach();
        self.navigate(Page::LibraryDetail(detail), cx);
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        // gpui-component requires the window's root view to mount these
        // layers; without them, modals / sheets / notifications won't
        // render anywhere.
        let sheet_layer = gpui_component::Root::render_sheet_layer(window, cx);
        let dialog_layer = gpui_component::Root::render_dialog_layer(window, cx);
        let notification_layer = gpui_component::Root::render_notification_layer(window, cx);

        div()
            .flex()
            .flex_col()
            .size_full()
            .font_family(theme::FONT_FAMILY)
            .text_color(theme.text)
            .text_size(px(14.0))
            .bg(theme.background)
            // Draggable custom title bar: window move + min/max/close controls
            // plus app-wide back/forward navigation and the current page title.
            .child(TitleBar::new().child(self.render_nav(cx)))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.0))
                    .w_full()
                    .overflow_hidden()
                    .child(self.render_sidenav(cx))
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            .child(self.render_content()),
                    ),
            )
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
    }
}
