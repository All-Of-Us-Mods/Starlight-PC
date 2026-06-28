use gpui::*;
use log::warn;

use crate::backend::api::{self, Server};
use crate::backend::services::region_service::{self, RegionInfo};
use crate::theme::ThemeExt;
use crate::views::{page_root, section_label};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::skeleton::Skeleton;
use gpui_component::{Icon, IconName, Sizable};

pub struct ServersView {
    state: LoadState,
    /// Current contents of Among Us' `regionInfo.json`, or `None` if it could
    /// not be read (e.g. non-Windows).
    regions: Option<RegionInfo>,
    notice: Option<String>,
    error: Option<String>,
}

enum LoadState {
    Loading,
    Loaded(Vec<Server>),
    Failed(String),
}

impl ServersView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self::load(cx);
        Self {
            state: LoadState::Loading,
            regions: None,
            notice: None,
            error: None,
        }
    }

    /// Fetch the server list and read the current region file together.
    fn load(cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let (servers, regions) = cx
                .background_executor()
                .spawn(async {
                    (
                        api::fetch_servers(),
                        region_service::read_region_info().ok(),
                    )
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.state = match servers {
                    Ok(servers) => LoadState::Loaded(servers),
                    Err(e) => LoadState::Failed(e.to_string()),
                };
                this.regions = regions;
                cx.notify();
            });
        })
        .detach();
    }

    fn reload_regions(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let regions = cx
                .background_executor()
                .spawn(async { region_service::read_region_info().ok() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.regions = regions;
                cx.notify();
            });
        })
        .detach();
    }

    fn add_server(&mut self, server: Server, cx: &mut Context<Self>) {
        self.error = None;
        self.notice = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let name = server.name.clone();
                    region_service::add_server_region(&server).map(|added| (name, added))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok((name, true)) => this.notice = Some(format!("Added region \"{name}\"")),
                    Ok((name, false)) => this.notice = Some(format!("\"{name}\" is already added")),
                    Err(e) => {
                        warn!("add region failed: {e}");
                        this.error = Some(e.to_string());
                    }
                }
                this.reload_regions(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn remove_region(&mut self, name: String, cx: &mut Context<Self>) {
        self.error = None;
        self.notice = None;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { region_service::remove_region(&name) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    warn!("remove region failed: {e}");
                    this.error = Some(e.to_string());
                }
                this.reload_regions(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn is_installed(&self, server: &Server) -> bool {
        self.regions.as_ref().is_some_and(|info| {
            info.regions
                .iter()
                .any(|r| region_service::region_has_server(r, &server.address, server.port))
        })
    }

    fn render_installed(&self, theme: &crate::theme::Theme, cx: &mut Context<Self>) -> AnyElement {
        let Some(info) = self.regions.as_ref() else {
            return div()
                .text_sm()
                .text_color(theme.text_muted)
                .child("Could not read regionInfo.json (Windows only).")
                .into_any_element();
        };

        if info.regions.is_empty() {
            return div()
                .text_sm()
                .text_color(theme.text_muted)
                .child("No regions configured yet. Add one below.")
                .into_any_element();
        }

        let rows = info.regions.iter().enumerate().map(|(ix, region)| {
            let name = region_service::region_name(region).to_string();
            let remove_name = name.clone();
            div()
                .flex()
                .items_center()
                .gap_3()
                .px_3()
                .py_2()
                .rounded_lg()
                .bg(theme.sidebar_background)
                .border_1()
                .border_color(theme.border)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_weight(FontWeight::MEDIUM)
                        .child(name),
                )
                .child(
                    Button::new(SharedString::from(format!("remove-region-{ix}")))
                        .ghost()
                        .xsmall()
                        .danger()
                        .icon(Icon::new(IconName::Delete))
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            this.remove_region(remove_name.clone(), cx)
                        })),
                )
                .into_any_element()
        });

        div()
            .flex()
            .flex_col()
            .gap_2()
            .children(rows)
            .into_any_element()
    }

    fn render_available(&self, theme: &crate::theme::Theme, cx: &mut Context<Self>) -> AnyElement {
        match &self.state {
            LoadState::Loading => div()
                .flex()
                .flex_col()
                .gap_2()
                .children((0..4).map(|_| {
                    Skeleton::new()
                        .w_full()
                        .h(px(56.0))
                        .rounded_lg()
                        .into_any_element()
                }))
                .into_any_element(),
            LoadState::Failed(e) => div()
                .text_color(theme.danger)
                .child(format!("Failed to load servers: {e}"))
                .into_any_element(),
            LoadState::Loaded(servers) if servers.is_empty() => div()
                .text_color(theme.text_muted)
                .child("No servers available.")
                .into_any_element(),
            LoadState::Loaded(servers) => {
                // Hide servers that are already configured (matched on host:port).
                let available: Vec<&Server> =
                    servers.iter().filter(|s| !self.is_installed(s)).collect();
                if available.is_empty() {
                    return div()
                        .text_sm()
                        .text_color(theme.text_muted)
                        .child("All available servers have been added.")
                        .into_any_element();
                }
                let rows = available.into_iter().map(|server| {
                    let server_for_add = server.clone();
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_3()
                        .py_2()
                        .rounded_lg()
                        .bg(theme.sidebar_background)
                        .border_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .truncate()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(server.name.clone()),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_xs()
                                        .text_color(theme.text_muted)
                                        .child(format!(
                                            "by {} · {}:{}",
                                            server.owner, server.address, server.port
                                        )),
                                ),
                        )
                        .child(
                            Button::new(SharedString::from(format!("add-server-{}", server.id)))
                                .primary()
                                .xsmall()
                                .icon(Icon::new(IconName::Plus))
                                .label("Add")
                                .on_click(cx.listener(move |this, _, _window, cx| {
                                    this.add_server(server_for_add.clone(), cx)
                                })),
                        )
                        .into_any_element()
                });
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(rows)
                    .into_any_element()
            }
        }
    }
}

impl Render for ServersView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        page_root("servers-page", &theme)
            .overflow_y_scroll()
            .gap_6()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_2xl().font_weight(FontWeight::BOLD).child("Servers"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child("Add community servers as Among Us regions. Restart the game to pick up changes."),
                    ),
            )
            .children(self.error.clone().map(|message| {
                div()
                    .rounded_md()
                    .bg(rgb(0x7f1d1d))
                    .p_3()
                    .text_color(theme.text)
                    .child(message)
            }))
            .children(
                self.notice
                    .clone()
                    .map(|message| div().text_sm().text_color(theme.success).child(message)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(section_label("Installed regions", &theme))
                    .child(self.render_installed(&theme, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(section_label("Available servers", &theme))
                    .child(self.render_available(&theme, cx)),
            )
    }
}
