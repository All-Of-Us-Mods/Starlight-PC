use gpui::prelude::FluentBuilder as _;
use gpui::*;
use log::warn;

use crate::backend::events::{self, BackendEvent};
use crate::backend::services::launch_service;
use crate::backend::services::profile_service::{self, ProfileEntry};
use crate::backend::state::game_runtime;
use crate::settings as app_settings;
use crate::theme::{self, ThemeExt};
use crate::ui::icon::AppIcon;
use crate::ui::stars_background::StarsBackground;
use crate::views::explore::ExploreView;
use crate::views::home::HomeView;
use crate::views::library::{LibraryEvent, LibraryView};
use crate::views::library_detail::{LibraryDetailEvent, LibraryDetailView};
use crate::views::lobbies::LobbiesView;
use crate::views::mod_detail::ModDetailView;
use crate::views::news_detail::NewsDetailView;
use crate::views::servers::ServersView;
use crate::views::settings::SettingsView;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::notification::Notification;
use gpui_component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarHeader, SidebarMenu, SidebarMenuItem, SidebarToggleButton,
};
use gpui_component::{Disableable, Icon, IconName, Sizable, TitleBar, WindowExt};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Home,
    Explore,
    Library,
    Servers,
    Lobbies,
    Settings,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Explore => "Explore",
            Tab::Library => "Library",
            Tab::Servers => "Servers",
            Tab::Lobbies => "Lobbies",
            Tab::Settings => "Settings",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Tab::Home => Icon::new(AppIcon::Home),
            Tab::Explore => Icon::new(AppIcon::Compass),
            Tab::Library => Icon::new(AppIcon::Library),
            Tab::Servers => Icon::new(IconName::Globe),
            Tab::Lobbies => Icon::new(IconName::LayoutDashboard),
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
    Servers,
    Lobbies,
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
    servers: Entity<ServersView>,
    lobbies: Entity<LobbiesView>,
    settings: Entity<SettingsView>,
    /// Most recently launched profile, shown as the title-bar quick-launch
    /// button. `None` until some profile has been launched at least once.
    last_launched: Option<ProfileEntry>,
    /// Live game-instance counts (from `GameStateChanged`); when `running_count`
    /// is non-zero the title-bar button becomes a red "Stop".
    running_count: usize,
    stoppable_count: usize,
    /// Icon-only sidebar (labels hidden). Session-only, not persisted.
    sidebar_collapsed: bool,
    /// Own entity so the drift animation only re-renders the starfield,
    /// not the whole workspace every frame.
    stars: Entity<StarsBackground>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let library = cx.new(|cx| LibraryView::new(window, cx));
        let home = cx.new(HomeView::new);
        let explore = cx.new(|cx| ExploreView::new(window, cx));
        let servers = cx.new(ServersView::new);
        let lobbies = cx.new(LobbiesView::new);
        let settings = cx.new(SettingsView::new);
        let initial = game_runtime::current_state();

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

        // Refresh the title-bar quick-launch target whenever a profile's launch
        // stats change (fired after every launch).
        let mut rx = events::subscribe();
        cx.spawn(async move |this, cx| {
            while let Ok(event) = rx.recv().await {
                match event {
                    BackendEvent::ProfileStatsUpdated(_) => {
                        let _ = this.update(cx, |_, cx| Self::reload_last_launched(cx));
                    }
                    BackendEvent::GameStateChanged(payload) => {
                        let _ = this.update(cx, |this, cx| {
                            this.running_count = payload.running_count;
                            this.stoppable_count = payload.stoppable_running_count;
                            cx.notify();
                        });
                    }
                    _ => {}
                }
            }
        })
        .detach();
        Self::reload_last_launched(cx);

        #[cfg(windows)]
        Self::check_for_update(window, cx);

        Self {
            history: vec![Page::Library],
            cursor: 0,
            library,
            home,
            explore,
            servers,
            lobbies,
            settings,
            last_launched: None,
            running_count: initial.running_count,
            stoppable_count: initial.stoppable_running_count,
            sidebar_collapsed: false,
            stars: cx.new(StarsBackground::new),
        }
    }

    /// Reload the most-recently-launched profile (for the title-bar launch
    /// button). `get_profiles` returns profiles sorted by last-launched first,
    /// so the first one that has actually been launched is the target.
    fn reload_last_launched(cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let profiles = cx
                .background_executor()
                .spawn(async { profile_service::get_profiles().unwrap_or_default() })
                .await;
            let last = profiles.into_iter().find(|p| p.last_launched_at.is_some());
            let _ = this.update(cx, |this, cx| {
                this.last_launched = last;
                cx.notify();
            });
        })
        .detach();
    }

    /// Check GitHub Releases for a newer build a few seconds after startup
    /// (so it doesn't compete with the initial UI render) and, if found,
    /// surface a notification offering to install it.
    #[cfg(windows)]
    fn check_for_update(window: &mut Window, cx: &mut Context<Self>) {
        use crate::backend::services::update_service;

        let window_handle = window.window_handle();
        cx.spawn(async move |_, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(3))
                .await;
            let update = cx
                .background_executor()
                .spawn(async { update_service::check_for_update() })
                .await;
            match update {
                Ok(Some(info)) => {
                    let _ = window_handle.update(cx, |_, window, cx| {
                        window.push_notification(update_notification(info), cx);
                    });
                }
                Ok(None) => {}
                Err(e) => warn!("update check failed: {e}"),
            }
        })
        .detach();
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
                Page::Servers => Some(Tab::Servers),
                Page::Lobbies => Some(Tab::Lobbies),
                Page::Settings => Some(Tab::Settings),
                Page::ModDetail(_) | Page::NewsDetail(_) => None,
            })
    }

    fn current_title(&self, cx: &App) -> SharedString {
        match self.current() {
            Page::Home => "Home".into(),
            Page::Explore => "Explore".into(),
            Page::Library => "Library".into(),
            Page::Servers => "Servers".into(),
            Page::Lobbies => "Lobbies".into(),
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
                | (Page::Servers, Tab::Servers)
                | (Page::Lobbies, Tab::Lobbies)
                | (Page::Settings, Tab::Settings)
        );
        if already_here {
            return;
        }
        let page = match tab {
            Tab::Home => Page::Home,
            Tab::Explore => Page::Explore,
            Tab::Library => Page::Library,
            Tab::Servers => Page::Servers,
            Tab::Lobbies => Page::Lobbies,
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
        let collapsed = self.sidebar_collapsed;

        // Collapsed: the 48px rail leaves ~32px inside the paddings, so use a
        // smaller icon and center it instead of the default space-between.
        let header = if collapsed {
            SidebarHeader::new().p_0().justify_center().child(
                Icon::new(AppIcon::Starlight)
                    .size(px(22.0))
                    .text_color(rgb(0xffc107)),
            )
        } else {
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
                )
        };

        Sidebar::new("starlight-sidebar")
            // Slimmer than the library's 255px default; keep in sync with
            // `SIDEBAR_WIDTH` in views/explore.rs (grid layout math).
            .w(px(175.0))
            .collapsible(SidebarCollapsible::Icon)
            .collapsed(collapsed)
            .header(header)
            .child(
                SidebarMenu::new()
                    .child(self.menu_item(Tab::Home, cx))
                    .child(self.menu_item(Tab::Explore, cx))
                    .child(self.menu_item(Tab::Library, cx))
                    .child(self.menu_item(Tab::Servers, cx))
                    .child(self.menu_item(Tab::Lobbies, cx))
                    .child(self.menu_item(Tab::Settings, cx)),
            )
            .footer(
                SidebarToggleButton::new()
                    .collapsed(collapsed)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.sidebar_collapsed = !this.sidebar_collapsed;
                        cx.notify();
                    })),
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
                            .small()
                            .icon(Icon::new(IconName::ArrowLeft))
                            .disabled(!self.can_go_back())
                            .on_click(cx.listener(|this, _, _window, cx| this.go_back(cx))),
                    )
                    .child(
                        Button::new("nav-forward")
                            .ghost()
                            .small()
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

    /// Title-bar action button: a red "Stop" while any game is running,
    /// otherwise "Launch <profile>" for the most recently launched profile
    /// (hidden until something has been launched).
    fn render_launch(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let button = if self.running_count > 0 {
            let label = if self.running_count > 1 {
                format!("Stop ({})", self.running_count)
            } else {
                "Stop".to_string()
            };
            let mut btn = Button::new("titlebar-launch")
                .danger()
                .small()
                .icon(Icon::new(IconName::Close))
                .label(label);
            if self.stoppable_count == 0 {
                // Only UWP instances are tracked — they can't be stopped here.
                btn = btn.disabled(true);
            } else {
                btn = btn.on_click(cx.listener(|this, _, window, cx| this.stop_all(window, cx)));
            }
            btn
        } else {
            let name = self.last_launched.as_ref()?.name.clone();
            Button::new("titlebar-launch")
                .primary()
                .small()
                .icon(Icon::new(IconName::Play))
                .label(format!("Launch {name}"))
                .on_click(cx.listener(|this, _, window, cx| this.launch_last(window, cx)))
        };
        Some(
            // Same drag-swallowing guard as the nav buttons (see render_nav).
            div()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button),
        )
    }

    /// Launch the most recently launched profile (modded) in the background,
    /// surfacing failures as a notification.
    fn launch_last(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(profile) = self.last_launched.clone() else {
            return;
        };
        let window_handle = window.window_handle();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { launch_service::launch_modded_for_profile(profile) })
                .await;
            if let Err(e) = result {
                warn!("Title-bar launch failed: {e}");
                let _ = window_handle.update(cx, |_, window, cx| {
                    window
                        .push_notification(Notification::error(format!("Launch failed: {e}")), cx);
                });
            }
        })
        .detach();
    }

    /// Stop all running game instances, surfacing failures as a notification.
    fn stop_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let window_handle = window.window_handle();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { game_runtime::stop_all_tracked_instances() })
                .await;
            if let Err(e) = result {
                warn!("Title-bar stop failed: {e}");
                let _ = window_handle.update(cx, |_, window, cx| {
                    window.push_notification(Notification::error(format!("Stop failed: {e}")), cx);
                });
            }
        })
        .detach();
    }

    fn render_content(&self) -> AnyElement {
        match self.current() {
            Page::Home => self.home.clone().into_any_element(),
            Page::Explore => self.explore.clone().into_any_element(),
            Page::Library => self.library.clone().into_any_element(),
            Page::Servers => self.servers.clone().into_any_element(),
            Page::Lobbies => self.lobbies.clone().into_any_element(),
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
            // from history rather than leaving a dangling "not found" page, and
            // refresh the quick-launch target in case it was the deleted one.
            LibraryDetailEvent::Close => {
                this.close_current(cx);
                Self::reload_last_launched(cx);
            }
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

        let show_stars = app_settings::get(cx).show_stars_background;

        div()
            .flex()
            .flex_col()
            .size_full()
            .relative()
            .font_family(theme::FONT_FAMILY)
            .text_color(theme.text)
            .text_size(px(14.0))
            .bg(theme.background)
            // Behind everything else. The sidebar and title bar are
            // transparent (see theme::apply), so the stars show through the
            // chrome; the content area paints over them.
            .when(show_stars, |el| {
                el.child(div().absolute().inset_0().child(self.stars.clone()))
            })
            // Draggable custom title bar: window move + min/max/close controls,
            // app-wide back/forward navigation, the current page title, and a
            // quick-launch button for the last launched profile.
            .child(
                TitleBar::new()
                    .child(self.render_nav(cx))
                    .children(self.render_launch(cx)),
            )
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
                            // Opaque: hides the starfield behind the page
                            // content, leaving stars visible only in the
                            // title bar and sidebar chrome (like upstream).
                            .bg(theme.background)
                            .child(self.render_content()),
                    ),
            )
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
    }
}

/// Build the "update available" notification, with an action button that
/// downloads the new exe, swaps it in, relaunches it, and quits the current
/// process.
#[cfg(windows)]
fn update_notification(info: crate::backend::services::update_service::UpdateInfo) -> Notification {
    Notification::info(format!("Starlight {} is available.", info.version))
        .title("Update available")
        .action(move |_, _, _| {
            let info = info.clone();
            Button::new("install-update")
                .label("Restart & Update")
                .primary()
                .on_click(move |_, window, cx| {
                    install_update(info.clone(), window, cx);
                })
        })
}

#[cfg(windows)]
fn install_update(
    info: crate::backend::services::update_service::UpdateInfo,
    window: &mut Window,
    cx: &mut App,
) {
    use crate::backend::services::update_service;

    let window_handle = window.window_handle();
    cx.spawn(async move |cx| {
        let result = cx
            .background_executor()
            .spawn(async move { update_service::apply_update_and_relaunch(&info) })
            .await;
        match result {
            Ok(()) => {
                cx.update(|cx| cx.quit());
            }
            Err(e) => {
                warn!("update install failed: {e}");
                let _ = window_handle.update(cx, |_, window, cx| {
                    window
                        .push_notification(Notification::error(format!("Update failed: {e}")), cx);
                });
            }
        }
    })
    .detach();
}
