use chrono::{DateTime, Local};
use gpui::*;
use gpui_component::{
    Disableable as _, Icon, IconName, WindowExt,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    notification::Notification,
    skeleton::Skeleton,
    tag::Tag,
};

use crate::ui::icon::AppIcon;
use log::warn;

use crate::backend::api::{self, ModResponse, ModVersion, ModVersionInfo};
use crate::backend::services::{
    mod_install_service::{self, InstallModInput, ResolvedDependency},
    profile_service::{self, ProfileEntry},
};
use crate::theme::{self, ThemeExt};
use crate::ui::mod_card::format_count;

#[derive(Clone, Debug)]
pub enum ModDetailEvent {
    Close,
}

impl EventEmitter<ModDetailEvent> for ModDetailView {}

pub struct ModDetailView {
    state: LoadState,
    profiles: Vec<ProfileEntry>,
    install: Option<InstallPanel>,
}

enum LoadState {
    Loading,
    Loaded(ModDetailData),
    Failed(String),
}

struct ModDetailData {
    mod_info: ModResponse,
    versions: Vec<ModVersion>,
    version_info: Option<ModVersionInfo>,
}

struct InstallPanel {
    selected_profile_id: Option<String>,
    deps: Vec<DepRow>,
    status: InstallStatus,
}

struct DepRow {
    mod_id: String,
    mod_name: String,
    resolved_version: String,
    constraint: String,
    dependency_type: String,
    /// Whether this dep is selected for install.
    checked: bool,
    /// Whether the currently-selected profile already has this mod at this version.
    already_installed: bool,
}

enum InstallStatus {
    Resolving,
    Ready,
    Installing(String),
    Done,
    Failed(String),
}

impl ModDetailView {
    pub fn new(mod_id: String, cx: &mut Context<Self>) -> Self {
        let view = Self {
            state: LoadState::Loading,
            profiles: Vec::new(),
            install: None,
        };
        cx.spawn(async move |this, cx| {
            let id = mod_id.clone();
            let result = cx
                .background_executor()
                .spawn(async move {
                    let mod_info = api::fetch_mod(&id)?;
                    let versions = api::fetch_mod_versions(&id).unwrap_or_default();
                    let version_info = versions.first().and_then(|version| {
                        api::fetch_mod_version_info(&id, &version.version).ok()
                    });
                    let profiles = profile_service::get_profiles().unwrap_or_default();
                    Ok::<(ModDetailData, Vec<ProfileEntry>), crate::backend::error::AppError>((
                        ModDetailData {
                            mod_info,
                            versions,
                            version_info,
                        },
                        profiles,
                    ))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok((data, profiles)) => {
                        this.state = LoadState::Loaded(data);
                        this.profiles = profiles;
                    }
                    Err(e) => this.state = LoadState::Failed(e.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
        view
    }

    fn open_install_panel(&mut self, cx: &mut Context<Self>) {
        let Some((mod_id, latest_version, deps)) = self.current_install_target() else {
            return;
        };
        let default_profile = self.profiles.first().map(|p| p.id.clone());
        self.install = Some(InstallPanel {
            selected_profile_id: default_profile.clone(),
            deps: Vec::new(),
            status: InstallStatus::Resolving,
        });
        cx.notify();

        cx.spawn(async move |this, cx| {
            let resolved = cx
                .background_executor()
                .spawn(async move { mod_install_service::resolve_dependencies(&deps).ok() })
                .await
                .unwrap_or_default();
            let _ = this.update(cx, |this, cx| {
                // Resolution may have raced the user picking a different profile;
                // use whatever's currently selected on the panel, not the snapshot
                // we captured when the panel opened.
                let selected = this
                    .install
                    .as_ref()
                    .and_then(|p| p.selected_profile_id.clone());
                let profile = selected
                    .as_ref()
                    .and_then(|id| this.profiles.iter().find(|p| &p.id == id));
                let rows = resolved
                    .into_iter()
                    .map(|r| dep_row_for(r, profile))
                    .collect::<Vec<_>>();
                if let Some(panel) = this.install.as_mut() {
                    panel.deps = rows;
                    panel.status = InstallStatus::Ready;
                }
                let _ = (mod_id, latest_version, default_profile);
                cx.notify();
            });
        })
        .detach();
    }

    fn current_install_target(
        &self,
    ) -> Option<(String, String, Vec<api::ModDependency>)> {
        let LoadState::Loaded(data) = &self.state else {
            return None;
        };
        let version = data.versions.first()?;
        let deps = data
            .version_info
            .as_ref()
            .map(|v| v.dependencies.clone())
            .unwrap_or_default();
        Some((data.mod_info.id.clone(), version.version.clone(), deps))
    }

    fn close_install_panel(&mut self, cx: &mut Context<Self>) {
        self.install = None;
        cx.notify();
    }

    fn select_profile(&mut self, profile_id: String, cx: &mut Context<Self>) {
        let Some(panel) = self.install.as_mut() else {
            return;
        };
        panel.selected_profile_id = Some(profile_id.clone());
        let profile = self.profiles.iter().find(|p| p.id == profile_id);
        for row in &mut panel.deps {
            let installed = profile
                .map(|p| profile_has_mod_at(p, &row.mod_id, &row.resolved_version))
                .unwrap_or(false);
            row.already_installed = installed;
            row.checked = !installed;
        }
        cx.notify();
    }

    fn toggle_dep(&mut self, ix: usize, checked: bool, cx: &mut Context<Self>) {
        if let Some(panel) = self.install.as_mut() {
            if let Some(row) = panel.deps.get_mut(ix) {
                row.checked = checked;
            }
        }
        cx.notify();
    }

    fn run_install(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.install.as_ref() else {
            return;
        };
        let Some(profile_id) = panel.selected_profile_id.clone() else {
            window.push_notification(Notification::warning("Pick a profile first"), cx);
            return;
        };
        let Some((mod_id, version, _)) = self.current_install_target() else {
            return;
        };

        // Install transitive deps first (already ordered deepest-first by the
        // resolver), then the root mod the user picked.
        let mut items: Vec<InstallModInput> = panel
            .deps
            .iter()
            .filter(|d| d.checked)
            .map(|d| InstallModInput {
                mod_id: d.mod_id.clone(),
                version: d.resolved_version.clone(),
            })
            .collect();
        items.push(InstallModInput { mod_id, version });

        if let Some(panel) = self.install.as_mut() {
            panel.status = InstallStatus::Installing("Installing BepInEx + mods…".into());
        }
        cx.notify();

        let profile_id_for_task = profile_id.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    profile_service::install_bepinex_for_profile(&profile_id_for_task)?;
                    mod_install_service::install_mods_for_profile(&profile_id_for_task, &items)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        if let Some(panel) = this.install.as_mut() {
                            panel.status = InstallStatus::Done;
                        }
                        this.profiles = profile_service::get_profiles().unwrap_or_default();
                    }
                    Err(e) => {
                        warn!("install_mods_for_profile failed: {e}");
                        if let Some(panel) = this.install.as_mut() {
                            panel.status = InstallStatus::Failed(e.to_string());
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}

fn profile_has_mod_at(profile: &ProfileEntry, mod_id: &str, version: &str) -> bool {
    profile
        .mods
        .iter()
        .any(|m| m.mod_id == mod_id && m.version == version)
}

fn dep_row_for(r: ResolvedDependency, profile: Option<&ProfileEntry>) -> DepRow {
    let installed = profile
        .map(|p| profile_has_mod_at(p, &r.mod_id, &r.resolved_version))
        .unwrap_or(false);
    let optional = r.dependency_type.eq_ignore_ascii_case("optional");
    DepRow {
        mod_id: r.mod_id,
        mod_name: r.mod_name,
        resolved_version: r.resolved_version,
        constraint: r.version_constraint,
        dependency_type: r.dependency_type,
        already_installed: installed,
        // Required deps default ON unless already installed; optional deps default OFF.
        checked: !installed && !optional,
    }
}

fn format_date(timestamp_ms: i64) -> String {
    DateTime::from_timestamp_millis(timestamp_ms)
        .map(|date| date.with_timezone(&Local).format("%b %-d, %Y").to_string())
        .unwrap_or_else(|| "Unknown date".to_string())
}

fn section_label(text: &'static str, theme: &crate::theme::Theme) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.text_muted)
        .child(text)
}

fn chip(text: String) -> impl IntoElement {
    Tag::new().child(text)
}

impl Render for ModDetailView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let back = Button::new("back")
            .ghost()
            .icon(Icon::new(IconName::ArrowLeft))
            .label("Back")
            .on_click(cx.listener(|_, _, _window, cx| {
                cx.emit(ModDetailEvent::Close);
            }));

        let body: AnyElement = match &self.state {
            LoadState::Loading => div()
                .flex()
                .flex_col()
                .gap_6()
                .child(
                    div().flex().justify_center().child(
                        Skeleton::new().w(px(176.0)).h(px(176.0)).rounded_lg(),
                    ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(Skeleton::new().w(px(220.0)).h(px(28.0)).rounded_md())
                        .child(Skeleton::new().w(px(120.0)).h_4().rounded_md()),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(Skeleton::new().w_full().h_3().rounded_md())
                        .child(Skeleton::new().w_full().h_3().rounded_md())
                        .child(Skeleton::new().w_5_6().h_3().rounded_md())
                        .child(Skeleton::new().w_3_4().h_3().rounded_md()),
                )
                .into_any_element(),
            LoadState::Failed(e) => div()
                .text_color(rgb(0xef4444))
                .child(format!("Failed: {e}"))
                .into_any_element(),
            LoadState::Loaded(data) => {
                let m = &data.mod_info;
                let latest_version_label = data
                    .versions
                    .first()
                    .map(|v| v.version.clone())
                    .unwrap_or_else(|| "—".to_string());
                let install_button = Button::new("install-mod")
                    .primary()
                    .icon(Icon::new(AppIcon::Download))
                    .label(format!("Install v{}", latest_version_label))
                    .on_click(cx.listener(|this, _, _window, cx| {
                        if this.install.is_some() {
                            this.close_install_panel(cx);
                        } else {
                            this.open_install_panel(cx);
                        }
                    }));

                let install_panel = self.install.as_ref().map(|panel| {
                    render_install_panel(panel, &self.profiles, &theme, cx)
                });

                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(
                        div().flex().justify_center().child(
                            img(api::mod_thumbnail_url(&m.id))
                                .w(px(176.0))
                                .h(px(176.0))
                                .object_fit(ObjectFit::Contain)
                                .rounded_lg()
                                .bg(theme.hover)
                                .border_1()
                                .border_color(theme.border),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child(m.name.clone()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text_muted)
                                    .child(format!("by {}", m.author)),
                            ),
                    )
                    .child(div().flex().justify_center().child(install_button))
                    .children(install_panel)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .justify_center()
                            .text_sm()
                            .gap_4()
                            .text_color(theme.text_muted)
                            .child(format!("{} downloads", format_count(m.downloads)))
                            .child(format!("Updated {}", format_date(m.updated_at)))
                            .children(m.mod_type.clone().map(|t| format!("Type: {t}"))),
                    )
                    .children(m.tags.clone().filter(|tags| !tags.is_empty()).map(|tags| {
                        div()
                            .flex()
                            .flex_wrap()
                            .justify_center()
                            .gap_2()
                            .children(tags.into_iter().map(chip))
                    }))
                    .child(
                        div()
                            .text_sm()
                            .line_height(px(22.0))
                            .text_color(theme.text_muted)
                            .child(m.description.clone()),
                    )
                    .child(div().h(px(1.0)).w_full().bg(theme.border))
                    .children(m.long_description.clone().map(|description| {
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(section_label("About", &theme))
                            .child(
                                div()
                                    .text_sm()
                                    .line_height(px(22.0))
                                    .text_color(theme.text)
                                    .child(description),
                            )
                    }))
                    .children(data.version_info.as_ref().and_then(|version| {
                        version.changelog.as_ref().map(|changelog| {
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .rounded_lg()
                                .bg(theme.hover)
                                .p_3()
                                .child(section_label("Changelog", &theme))
                                .child(
                                    div()
                                        .text_sm()
                                        .line_height(px(22.0))
                                        .child(changelog.clone()),
                                )
                        })
                    }))
                    .children(
                        m.links
                            .clone()
                            .filter(|links| !links.is_empty())
                            .map(|links| {
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(section_label("Links", &theme))
                                    .child(div().flex().flex_wrap().gap_2().children(
                                        links.into_iter().map(|link| {
                                            chip(format!("{}: {}", link.link_type, link.url))
                                        }),
                                    ))
                            }),
                    )
                    .children(m.license.clone().map(|license| {
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(format!("Licensed under {license}"))
                    }))
                    .into_any_element()
            }
        };

        div()
            .id("mod-detail-page")
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .overflow_y_scroll()
            .font_family(theme::FONT_FAMILY)
            .text_color(theme.text)
            .text_size(px(14.0))
            .p_8()
            .pt(px(48.0))
            .child(back)
            .child(body)
    }
}

fn render_install_panel(
    panel: &InstallPanel,
    profiles: &[ProfileEntry],
    theme: &crate::theme::Theme,
    cx: &mut Context<ModDetailView>,
) -> AnyElement {
    let status_row = match &panel.status {
        InstallStatus::Resolving => Some(
            div()
                .text_xs()
                .text_color(theme.text_muted)
                .child("Resolving dependencies…")
                .into_any_element(),
        ),
        InstallStatus::Installing(msg) => Some(
            div()
                .text_xs()
                .text_color(theme.text_muted)
                .child(msg.clone())
                .into_any_element(),
        ),
        InstallStatus::Done => Some(
            div()
                .text_xs()
                .text_color(rgb(0x22c55e))
                .child("Installed.")
                .into_any_element(),
        ),
        InstallStatus::Failed(e) => Some(
            div()
                .text_xs()
                .text_color(rgb(0xef4444))
                .child(format!("Failed: {e}"))
                .into_any_element(),
        ),
        InstallStatus::Ready => None,
    };

    let busy = matches!(panel.status, InstallStatus::Installing(_) | InstallStatus::Resolving);

    let profile_rows = profiles.iter().enumerate().map(|(ix, p)| {
        let selected = panel.selected_profile_id.as_deref() == Some(p.id.as_str());
        let id = p.id.clone();
        div()
            .id(SharedString::from(format!("install-profile-{ix}")))
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(if selected { theme.primary } else { theme.border })
            .bg(if selected { theme.hover } else { theme.background })
            .cursor_pointer()
            .child(p.name.clone())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _window, cx| {
                    this.select_profile(id.clone(), cx);
                }),
            )
    });

    let dep_rows = panel.deps.iter().enumerate().map(|(ix, row)| {
        let label = format!(
            "{} v{}{}",
            row.mod_name,
            row.resolved_version,
            if row.dependency_type.eq_ignore_ascii_case("optional") {
                " (optional)"
            } else {
                ""
            }
        );
        let detail = format!(
            "{} — constraint {}",
            row.mod_id,
            if row.constraint.is_empty() {
                "*"
            } else {
                row.constraint.as_str()
            }
        );
        let already = row.already_installed;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                Checkbox::new(SharedString::from(format!("dep-{ix}")))
                    .checked(row.checked)
                    .label(label)
                    .disabled(already || busy)
                    .on_click(cx.listener(move |this, checked: &bool, _window, cx| {
                        this.toggle_dep(ix, *checked, cx);
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .pl_6()
                    .child(if already {
                        format!("{detail} — already installed")
                    } else {
                        detail
                    }),
            )
    });

    let close_btn = Button::new("install-close")
        .ghost()
        .label("Close")
        .on_click(cx.listener(|this, _, _window, cx| {
            this.close_install_panel(cx);
        }));
    let install_btn = Button::new("install-confirm")
        .primary()
        .icon(Icon::new(AppIcon::Download))
        .label("Install")
        .disabled(busy || panel.selected_profile_id.is_none())
        .on_click(cx.listener(|this, _, window, cx| {
            this.run_install(window, cx);
        }));

    div()
        .flex()
        .flex_col()
        .gap_4()
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.hover)
        .child(section_label("Install into profile", theme))
        .child(if profiles.is_empty() {
            div()
                .text_sm()
                .text_color(theme.text_muted)
                .child("No profiles yet. Create one from the Library tab first.")
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .children(profile_rows)
                .into_any_element()
        })
        .children(if panel.deps.is_empty() {
            None
        } else {
            Some(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(section_label("Dependencies", theme))
                    .children(dep_rows),
            )
        })
        .children(status_row)
        .child(
            div()
                .flex()
                .justify_end()
                .gap_2()
                .child(close_btn)
                .child(install_btn),
        )
        .into_any_element()
}
