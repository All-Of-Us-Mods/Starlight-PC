use gpui::*;

use crate::theme::{self, ThemeExt};
use crate::ui::icon::AppIcon;
use crate::views::explore::ExploreView;
use crate::views::home::HomeView;
use crate::views::library::{LibraryEvent, LibraryView};
use crate::views::library_detail::{LibraryDetailEvent, LibraryDetailView};
use crate::views::mod_detail::{ModDetailEvent, ModDetailView};
use crate::views::news_detail::{NewsDetailEvent, NewsDetailView};
use crate::views::settings::SettingsView;
use gpui_component::sidebar::{Sidebar, SidebarHeader, SidebarMenu, SidebarMenuItem};
use gpui_component::{Icon, IconName};

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

enum ActiveView {
    Home(Entity<HomeView>),
    Explore(Entity<ExploreView>),
    Library,
    LibraryDetail(Entity<LibraryDetailView>),
    ModDetail(Entity<ModDetailView>),
    NewsDetail(Entity<NewsDetailView>),
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
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let library = cx.new(|cx| LibraryView::new(window, cx));
        let home = cx.new(|cx| HomeView::new(cx));
        let explore = cx.new(|cx| ExploreView::new(window, cx));
        let settings = cx.new(|cx| SettingsView::new(cx));

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
                LibraryEvent::Open(profile_id) => {
                    let id = profile_id.clone();
                    let detail = cx.new(|cx| LibraryDetailView::new(id, window, cx));
                    cx.subscribe(&detail, |this, _, ev: &LibraryDetailEvent, cx| match ev {
                        LibraryDetailEvent::Close => {
                            this.library.update(cx, |lib, cx| lib.refresh(cx));
                            this.view = ActiveView::Library;
                            cx.notify();
                        }
                    })
                    .detach();
                    this.view = ActiveView::LibraryDetail(detail);
                    cx.notify();
                }
            },
        )
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
            Tab::Library => {
                // Profiles can change from anywhere (mod install, BepInEx install,
                // …); pull a fresh list whenever the user navigates back here.
                self.library.update(cx, |lib, cx| lib.refresh(cx));
                ActiveView::Library
            }
            Tab::Settings => ActiveView::Settings(self.settings.clone()),
        };
        cx.notify();
    }

    fn menu_item(&self, tab: Tab, cx: &mut Context<Self>) -> SidebarMenuItem {
        SidebarMenuItem::new(tab.label())
            .icon(tab.icon())
            .active(self.tab == tab)
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

    fn render_content(&self) -> AnyElement {
        match &self.view {
            ActiveView::Home(v) => v.clone().into_any_element(),
            ActiveView::Explore(v) => v.clone().into_any_element(),
            ActiveView::Library => self.library.clone().into_any_element(),
            ActiveView::LibraryDetail(v) => v.clone().into_any_element(),
            ActiveView::ModDetail(v) => v.clone().into_any_element(),
            ActiveView::NewsDetail(v) => v.clone().into_any_element(),
            ActiveView::Settings(v) => v.clone().into_any_element(),
        }
    }

    fn open_mod(&mut self, mod_id: String, cx: &mut Context<Self>) {
        let detail = cx.new(|cx| ModDetailView::new(mod_id, cx));
        let return_tab = self.tab;
        cx.subscribe(&detail, move |this, _, ev: &ModDetailEvent, cx| match ev {
            ModDetailEvent::Close => {
                this.switch_tab(return_tab, cx);
            }
        })
        .detach();
        self.view = ActiveView::ModDetail(detail);
        cx.notify();
    }

    fn open_news(&mut self, post: crate::backend::api::Post, cx: &mut Context<Self>) {
        let detail = cx.new(|_| NewsDetailView::new(post));
        let return_tab = self.tab;
        cx.subscribe(&detail, move |this, _, ev: &NewsDetailEvent, cx| match ev {
            NewsDetailEvent::Close => {
                this.switch_tab(return_tab, cx);
            }
        })
        .detach();
        self.view = ActiveView::NewsDetail(detail);
        cx.notify();
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
            .size_full()
            .font_family(theme::FONT_FAMILY)
            .text_color(theme.text)
            .text_size(px(14.0))
            .bg(theme.background)
            .child(self.render_sidenav(cx))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .overflow_hidden()
                    .child(self.render_content()),
            )
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
    }
}
