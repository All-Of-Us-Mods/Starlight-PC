use gpui::*;
use gpui_component::{
    Disableable as _, Icon, WindowExt,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    input::{Input, InputState},
    notification::Notification,
    progress::Progress,
    skeleton::Skeleton,
    tag::Tag,
};

use crate::ui::icon::AppIcon;
use log::warn;

use crate::backend::api::{self, ModResponse, ModVersion, ModVersionInfo};
use crate::backend::events::{self, BackendEvent};
use crate::backend::services::{
    bepinex_service::BepInExTargetType,
    mod_install_service::{self, InstallModInput, ResolvedDependency},
    profile_service::{self, ProfileEntry},
};
use crate::theme::ThemeExt;
use crate::ui::format;
use crate::ui::mod_card::format_count;
use crate::views::{page_root, section_label};

pub struct ModDetailView {
    state: LoadState,
    profiles: Vec<ProfileEntry>,
    install: Option<InstallPanel>,
}

enum LoadState {
    Loading,
    Loaded(Box<ModDetailData>),
    Failed(String),
}

struct ModDetailData {
    mod_info: ModResponse,
    versions: Vec<ModVersion>,
    version_info: Option<ModVersionInfo>,
}

struct InstallPanel {
    selected_profile_id: Option<String>,
    selected_version: String,
    deps: Vec<DepRow>,
    unresolved: Vec<String>,
    status: InstallStatus,
    new_profile: Option<NewProfileInput>,
}

struct NewProfileInput {
    name_input: Entity<InputState>,
    busy: bool,
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
    Installing { message: String, progress: f32 },
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
                        this.state = LoadState::Loaded(Box::new(data));
                        this.profiles = profiles;
                    }
                    Err(e) => this.state = LoadState::Failed(e.to_string()),
                }
                cx.notify();
            });
        })
        .detach();

        // Feed BepInEx + mod download progress into the install panel while
        // an install is in flight.
        let mut rx = events::subscribe();
        cx.spawn(async move |this, cx| {
            while let Ok(event) = rx.recv().await {
                let _ = this.update(cx, |this, cx| {
                    apply_progress_event(this, event);
                    cx.notify();
                });
            }
        })
        .detach();

        view
    }

    /// Title shown in the app title bar — the mod name once loaded.
    pub fn title(&self) -> SharedString {
        match &self.state {
            LoadState::Loaded(data) => data.mod_info.name.clone().into(),
            _ => "Mod".into(),
        }
    }

    fn open_install_panel(&mut self, cx: &mut Context<Self>) {
        let LoadState::Loaded(data) = &self.state else {
            return;
        };
        let Some(latest) = data.versions.first() else {
            return;
        };
        let mod_id = data.mod_info.id.clone();
        let latest_version = latest.version.clone();
        let default_profile = self.profiles.first().map(|p| p.id.clone());
        self.install = Some(InstallPanel {
            selected_profile_id: default_profile,
            selected_version: latest_version.clone(),
            deps: Vec::new(),
            unresolved: Vec::new(),
            status: InstallStatus::Resolving,
            new_profile: None,
        });
        cx.notify();
        self.resolve_for_selected_version(mod_id, latest_version, cx);
    }

    /// Restart dependency resolution after a failed install, clearing the
    /// half-updated panel state the failure may have left behind.
    fn retry_install_resolve(&mut self, cx: &mut Context<Self>) {
        let LoadState::Loaded(data) = &self.state else {
            return;
        };
        let mod_id = data.mod_info.id.clone();
        let Some(panel) = self.install.as_mut() else {
            return;
        };
        let version = panel.selected_version.clone();
        panel.status = InstallStatus::Resolving;
        panel.deps.clear();
        panel.unresolved.clear();
        cx.notify();
        self.resolve_for_selected_version(mod_id, version, cx);
    }

    fn resolve_for_selected_version(
        &mut self,
        mod_id: String,
        version: String,
        cx: &mut Context<Self>,
    ) {
        let version_for_task = version.clone();
        cx.spawn(async move |this, cx| {
            let (resolved, unresolved) = cx
                .background_executor()
                .spawn(async move {
                    let info = api::fetch_mod_version_info(&mod_id, &version_for_task).ok();
                    let deps = info.map(|i| i.dependencies).unwrap_or_default();
                    mod_install_service::resolve_dependencies(&deps).ok()
                })
                .await
                .unwrap_or_default();
            let _ = this.update(cx, |this, cx| {
                // Stale-callback guards: only apply if the panel still exists and
                // is still showing the same version we resolved for.
                let still_relevant = this
                    .install
                    .as_ref()
                    .is_some_and(|p| p.selected_version == version);
                if !still_relevant {
                    return;
                }
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
                    panel.unresolved = unresolved;
                    panel.status = InstallStatus::Ready;
                }
                cx.notify();
            });
        })
        .detach();
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
            let installed =
                profile.is_some_and(|p| profile_has_mod_at(p, &row.mod_id, &row.resolved_version));
            row.already_installed = installed;
            row.checked = !installed;
        }
        cx.notify();
    }

    fn select_version(&mut self, version: String, cx: &mut Context<Self>) {
        let mod_id = match &self.state {
            LoadState::Loaded(data) => data.mod_info.id.clone(),
            _ => return,
        };
        let Some(panel) = self.install.as_mut() else {
            return;
        };
        if panel.selected_version == version {
            return;
        }
        panel.selected_version = version.clone();
        panel.deps.clear();
        panel.unresolved.clear();
        panel.status = InstallStatus::Resolving;
        cx.notify();
        self.resolve_for_selected_version(mod_id, version, cx);
    }

    fn toggle_new_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.install.as_mut() else {
            return;
        };
        if panel.new_profile.is_some() {
            panel.new_profile = None;
        } else {
            let name_input =
                cx.new(|cx| InputState::new(window, cx).placeholder("New profile name"));
            panel.new_profile = Some(NewProfileInput {
                name_input,
                busy: false,
            });
        }
        cx.notify();
    }

    fn submit_new_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.install.as_mut() else {
            return;
        };
        let Some(new_profile) = panel.new_profile.as_mut() else {
            return;
        };
        if new_profile.busy {
            return;
        }
        let name = new_profile.name_input.read(cx).value().to_string();
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            window.push_notification(Notification::warning("Profile name cannot be empty"), cx);
            return;
        }
        new_profile.busy = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::create_profile(&trimmed) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(profile) => {
                        this.profiles = profile_service::get_profiles().unwrap_or_default();
                        if let Some(panel) = this.install.as_mut() {
                            panel.new_profile = None;
                        }
                        // Select the newly created profile (this also re-evaluates
                        // dep checkboxes against its empty mod list).
                        this.select_profile(profile.id, cx);
                    }
                    Err(e) => {
                        warn!("create_profile failed: {e}");
                        if let Some(panel) = this.install.as_mut()
                            && let Some(np) = panel.new_profile.as_mut()
                        {
                            np.busy = false;
                        }
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn toggle_dep(&mut self, ix: usize, checked: bool, cx: &mut Context<Self>) {
        if let Some(panel) = self.install.as_mut()
            && let Some(row) = panel.deps.get_mut(ix)
        {
            row.checked = checked;
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
        let mod_id = match &self.state {
            LoadState::Loaded(data) => data.mod_info.id.clone(),
            _ => return,
        };
        let version = panel.selected_version.clone();

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

        let needs_bepinex = self
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .is_none_or(|p| p.bepinex_installed != Some(true));
        if let Some(panel) = self.install.as_mut() {
            panel.status = InstallStatus::Installing {
                message: if needs_bepinex {
                    "Installing BepInEx + mods…"
                } else {
                    "Installing mods…"
                }
                .into(),
                progress: 0.0,
            };
        }
        cx.notify();

        let profile_id_for_task = profile_id.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if needs_bepinex {
                        profile_service::install_bepinex_for_profile(&profile_id_for_task)?;
                    }
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

fn apply_progress_event(this: &mut ModDetailView, event: BackendEvent) {
    let Some(panel) = this.install.as_mut() else {
        return;
    };
    let InstallStatus::Installing { message, progress } = &mut panel.status else {
        return;
    };
    match event {
        BackendEvent::BepInExProgress(p)
            if matches!(p.target_type, BepInExTargetType::Profile)
                && Some(p.target_id.as_str()) == panel.selected_profile_id.as_deref() =>
        {
            *message = format!("BepInEx: {}", p.message);
            *progress = p.progress as f32;
        }
        BackendEvent::ModDownloadProgress(p) => {
            *message = format!("{} ({})", p.mod_id, p.stage);
            *progress = p.progress as f32;
        }
        _ => {}
    }
}

fn profile_has_mod_at(profile: &ProfileEntry, mod_id: &str, version: &str) -> bool {
    profile
        .mods
        .iter()
        .any(|m| m.mod_id == mod_id && m.version == version)
}

fn dep_row_for(r: ResolvedDependency, profile: Option<&ProfileEntry>) -> DepRow {
    let installed = profile.is_some_and(|p| profile_has_mod_at(p, &r.mod_id, &r.resolved_version));
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

fn chip(text: String) -> impl IntoElement {
    Tag::new().child(text)
}

impl Render for ModDetailView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let body: AnyElement = match &self.state {
            LoadState::Loading => div()
                .flex()
                .flex_col()
                .gap_6()
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .child(Skeleton::new().w(px(176.0)).h(px(176.0)).rounded_lg()),
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
                .text_color(theme.danger)
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
                    render_install_panel(panel, &self.profiles, &data.versions, &theme, cx)
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
                            .child(format!("Updated {}", format::date_ms(m.updated_at)))
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

        page_root("mod-detail-page", &theme)
            .gap_4()
            .overflow_y_scroll()
            .child(body)
    }
}

fn render_install_panel(
    panel: &InstallPanel,
    profiles: &[ProfileEntry],
    versions: &[ModVersion],
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
        InstallStatus::Installing { message, progress } => Some(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_muted)
                        .child(message.clone()),
                )
                .child(Progress::new("install-progress").value(*progress))
                .into_any_element(),
        ),
        InstallStatus::Done => Some(
            div()
                .text_xs()
                .text_color(theme.success)
                .child("Installed.")
                .into_any_element(),
        ),
        InstallStatus::Failed(e) => Some(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.danger)
                        .child(format!("Failed: {e}")),
                )
                .child(
                    // Re-resolve from scratch — a failed install can leave the
                    // panel's dependency state half-updated.
                    Button::new("install-retry").label("Retry").on_click(
                        cx.listener(|this, _, _window, cx| this.retry_install_resolve(cx)),
                    ),
                )
                .into_any_element(),
        ),
        InstallStatus::Ready => None,
    };

    let busy = matches!(
        panel.status,
        InstallStatus::Installing { .. } | InstallStatus::Resolving
    );

    let profile_rows = profiles.iter().enumerate().map(|(ix, p)| {
        let selected = panel.selected_profile_id.as_deref() == Some(p.id.as_str());
        let id = p.id.clone();
        div()
            .id(SharedString::from(format!("install-profile-{ix}")))
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                theme.primary
            } else {
                theme.border
            })
            .bg(if selected {
                theme.hover
            } else {
                theme.background
            })
            .cursor_pointer()
            .child(p.name.clone())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _window, cx| {
                    this.select_profile(id.clone(), cx);
                }),
            )
            .into_any_element()
    });
    let new_profile_chip = div()
        .id("install-profile-new")
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .cursor_pointer()
        .child("+ New profile")
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _, window, cx| {
                this.toggle_new_profile(window, cx);
            }),
        )
        .into_any_element();
    let profile_chips: Vec<AnyElement> = profile_rows
        .chain(std::iter::once(new_profile_chip))
        .collect();

    let version_rows = versions.iter().enumerate().map(|(ix, v)| {
        let selected = panel.selected_version == v.version;
        let version = v.version.clone();
        div()
            .id(SharedString::from(format!("install-version-{ix}")))
            .px_3()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                theme.primary
            } else {
                theme.border
            })
            .bg(if selected {
                theme.hover
            } else {
                theme.background
            })
            .cursor_pointer()
            .text_xs()
            .child(format!("v{}", v.version))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _window, cx| {
                    this.select_version(version.clone(), cx);
                }),
            )
    });

    let new_profile_row = panel.new_profile.as_ref().map(|np| {
        let busy = np.busy;
        div()
            .flex()
            .gap_2()
            .items_center()
            .child(Input::new(&np.name_input).w(px(220.0)))
            .child(
                Button::new("new-profile-create")
                    .primary()
                    .label(if busy { "Creating…" } else { "Create" })
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.submit_new_profile(window, cx);
                    })),
            )
            .child(
                Button::new("new-profile-cancel")
                    .ghost()
                    .label("Cancel")
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.toggle_new_profile(window, cx);
                    })),
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
        .child(div().flex().flex_wrap().gap_2().children(profile_chips))
        .children(new_profile_row)
        .child(section_label("Version", theme))
        .child(div().flex().flex_wrap().gap_2().children(version_rows))
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
        .children(if panel.unresolved.is_empty() {
            None
        } else {
            Some(div().text_xs().text_color(theme.text_muted).child(format!(
                "{} dependencies could not be resolved and will be skipped: {}",
                panel.unresolved.len(),
                panel.unresolved.join(", ")
            )))
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
