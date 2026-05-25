//! Profile detail page. Composes the launch / metadata controls with the
//! reusable [`LogPanel`] (in `ui/log_panel`) and the icon picker dialog (in
//! `icon_dialog`). Long-running work — disk reads, API lookups, launch —
//! always happens on the background executor; this module only orchestrates.

mod icon_dialog;

use gpui::*;
use log::warn;

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::{LazyLock, Mutex};

use crate::backend::api;
use crate::backend::events::{self, BackendEvent};
use crate::backend::services::bepinex_service::{BepInExProgress, BepInExTargetType};
use crate::backend::services::launch_service;
use crate::backend::services::profile_service::{self, ProfileEntry, ProfileModEntry};
use crate::backend::state::game_runtime;
use crate::settings as app_settings;
use crate::theme::{self, ThemeExt};
use crate::ui::format;
use crate::ui::icon::AppIcon;
use crate::ui::log_panel::LogPanel;
use crate::ui::profile_icon::profile_icon;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::progress::Progress;
use gpui_component::skeleton::Skeleton;
use gpui_component::{Disableable, Icon, IconName};

use icon_dialog::{IconDialogState, render_icon_dialog};

static MOD_NAME_CACHE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Debug)]
pub enum LibraryDetailEvent {
    Close,
}

impl EventEmitter<LibraryDetailEvent> for LibraryDetailView {}

pub struct LibraryDetailView {
    pub(super) profile_id: String,
    pub(super) state: LoadState,
    bep_progress: Option<BepInExProgress>,
    confirming_delete: bool,
    launch_error: Option<String>,
    rename_dialog: Option<Entity<InputState>>,
    export_dialog: Option<Entity<InputState>>,
    pub(super) icon_dialog: Option<IconDialogState>,
    running_count: usize,
    stoppable_count: usize,
    log_panel: Entity<LogPanel>,
    /// API-resolved display names per mod_id, populated lazily after load.
    mod_names: HashMap<String, String>,
}

pub(super) enum LoadState {
    Loading,
    Loaded(ProfileEntry),
    NotFound,
    Failed(String),
}

impl LibraryDetailView {
    pub fn new(profile_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let log_panel = cx.new(|cx| LogPanel::new(window, cx));

        let view = Self {
            profile_id: profile_id.clone(),
            state: LoadState::Loading,
            bep_progress: None,
            confirming_delete: false,
            launch_error: None,
            rename_dialog: None,
            export_dialog: None,
            icon_dialog: None,
            running_count: 0,
            stoppable_count: 0,
            log_panel,
            mod_names: cached_mod_names(),
        };

        view.spawn_load(cx);

        // Subscribe to backend events for *this* profile.
        let id_for_events = profile_id.clone();
        let mut rx = events::subscribe();
        cx.spawn(async move |this, cx| {
            while let Ok(event) = rx.recv().await {
                match event {
                    BackendEvent::BepInExProgress(p)
                        if matches!(p.target_type, BepInExTargetType::Profile)
                            && p.target_id == id_for_events =>
                    {
                        let done = p.stage == "complete";
                        let _ = this.update(cx, |this, cx| {
                            this.bep_progress = if done { None } else { Some(p) };
                            cx.notify();
                        });
                        if done {
                            let _ = this.update(cx, |this, cx| this.spawn_load(cx));
                        }
                    }
                    BackendEvent::GameStateChanged(payload) => {
                        let running = payload
                            .profile_instance_counts
                            .get(&id_for_events)
                            .copied()
                            .unwrap_or(0);
                        let stoppable = payload
                            .stoppable_profile_instance_counts
                            .get(&id_for_events)
                            .copied()
                            .unwrap_or(0);
                        let _ = this.update(cx, |this, cx| {
                            this.running_count = running;
                            this.stoppable_count = stoppable;
                            cx.notify();
                            // Game state change ≈ new log content / mod changes.
                            this.refresh_disk_state(cx);
                        });
                    }
                    BackendEvent::ProfileStatsUpdated(id) if id == id_for_events => {
                        let _ = this.update(cx, |this, cx| this.spawn_load(cx));
                    }
                    _ => {}
                }
            }
        })
        .detach();

        view
    }

    pub(super) fn spawn_load(&self, cx: &mut Context<Self>) {
        let id = self.profile_id.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::get_profile_by_id(&id) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.state = match result {
                    Ok(Some(p)) => LoadState::Loaded(p),
                    Ok(None) => LoadState::NotFound,
                    Err(e) => LoadState::Failed(e.to_string()),
                };
                cx.notify();
                this.refresh_disk_state(cx);
                this.fetch_missing_mod_names(cx);
            });
        })
        .detach();
    }

    /// Resolve API display names for any mods we haven't seen yet. Successful
    /// lookups are shared across detail-page instances for the app session.
    fn fetch_missing_mod_names(&self, cx: &mut Context<Self>) {
        let LoadState::Loaded(profile) = &self.state else {
            return;
        };
        let cached = cached_mod_names();
        let pending: Vec<String> = profile
            .mods
            .iter()
            .map(|m| m.mod_id.clone())
            .filter(|id| !cached.contains_key(id) && !self.mod_names.contains_key(id))
            .collect();
        if !cached.is_empty() {
            cx.spawn(async move |this, cx| {
                let _ = this.update(cx, |this, cx| {
                    this.mod_names.extend(cached);
                    cx.notify();
                });
            })
            .detach();
        }
        if pending.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            for mod_id in pending {
                let id_for_fetch = mod_id.clone();
                let resolved = cx
                    .background_executor()
                    .spawn(async move { api::fetch_mod(&id_for_fetch).ok().map(|m| m.name) })
                    .await;
                if let Some(name) = resolved {
                    let _ = this.update(cx, |this, cx| {
                        cache_mod_name(mod_id.clone(), name.clone());
                        this.mod_names.insert(mod_id, name);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    /// Reload the on-disk log shown on this page. Cheap to call repeatedly —
    /// runs on the background executor and pushes the new content into the
    /// `LogPanel` entity, so we never touch the disk inside `render()`.
    fn refresh_disk_state(&self, cx: &mut Context<Self>) {
        let LoadState::Loaded(profile) = &self.state else {
            return;
        };
        let path = profile.path.clone();
        let log_panel = self.log_panel.clone();
        cx.spawn(async move |_this, cx| {
            let log = cx
                .background_executor()
                .spawn(async move { profile_service::get_profile_log(&path, "LogOutput.log") })
                .await;
            log_panel.update(cx, |panel, cx| {
                panel.set_content(log, cx);
            });
        })
        .detach();
    }

    fn install_bepinex(&mut self, cx: &mut Context<Self>) {
        let id = self.profile_id.clone();
        cx.background_executor()
            .spawn(async move {
                if let Err(e) = profile_service::install_bepinex_for_profile(&id) {
                    warn!("install_bepinex_for_profile failed: {e}");
                }
            })
            .detach();
    }

    fn open_profile_folder(&self) {
        let LoadState::Loaded(profile) = &self.state else {
            return;
        };
        if let Err(e) = open_folder(Path::new(&profile.path)) {
            warn!("open profile folder failed: {e}");
        }
    }

    fn launch(&mut self, cx: &mut Context<Self>) {
        let LoadState::Loaded(profile) = &self.state else {
            return;
        };
        let profile = profile.clone();
        self.launch_error = None;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { launch_service::launch_modded_for_profile(profile) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    warn!("launch failed: {e}");
                    this.launch_error = Some(e.to_string());
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        let id = self.profile_id.clone();
        self.launch_error = None;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { game_runtime::stop_profile_instances(&id) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    warn!("stop failed: {e}");
                    this.launch_error = Some(e.to_string());
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn delete_profile(&mut self, cx: &mut Context<Self>) {
        let id = self.profile_id.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::delete_profile(&id) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    warn!("delete_profile failed: {e}");
                    this.confirming_delete = false;
                    cx.notify();
                } else {
                    cx.emit(LibraryDetailEvent::Close);
                }
            });
        })
        .detach();
    }

    fn open_rename_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = match &self.state {
            LoadState::Loaded(profile) => profile.name.clone(),
            _ => String::new(),
        };
        let state = cx.new(|cx| {
            let mut s = InputState::new(window, cx).placeholder("Profile name");
            s.set_value(name, window, cx);
            s
        });
        state.read(cx).focus_handle(cx).focus(window, cx);
        cx.subscribe_in(
            &state,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    this.submit_rename(state.read(cx).value().to_string(), cx);
                }
            },
        )
        .detach();
        self.rename_dialog = Some(state);
        cx.notify();
    }

    fn submit_rename(&mut self, name: String, cx: &mut Context<Self>) {
        let id = self.profile_id.clone();
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        self.rename_dialog = None;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::rename_profile(&id, &name) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    this.launch_error = Some(format!("Rename failed: {e}"));
                    cx.notify();
                }
                this.spawn_load(cx);
            });
        })
        .detach();
    }

    fn open_export_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = cx.new(|cx| InputState::new(window, cx).placeholder("Destination .zip path"));
        state.read(cx).focus_handle(cx).focus(window, cx);
        cx.subscribe_in(
            &state,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    this.submit_export(state.read(cx).value().to_string(), cx);
                }
            },
        )
        .detach();
        self.export_dialog = Some(state);
        cx.notify();
    }

    fn submit_export(&mut self, path: String, cx: &mut Context<Self>) {
        let id = self.profile_id.clone();
        let path = path.trim().to_string();
        if path.is_empty() {
            return;
        }
        self.export_dialog = None;
        let success_path = path.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::export_profile_zip(&id, &path) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    this.launch_error = Some(format!("Export failed: {e}"));
                } else {
                    this.launch_error = Some(format!("Exported profile to {success_path}"));
                }
                cx.notify();
            });
        })
        .detach();
    }
}

fn cached_mod_names() -> HashMap<String, String> {
    MOD_NAME_CACHE
        .lock()
        .map(|cache| cache.clone())
        .unwrap_or_default()
}

fn cache_mod_name(mod_id: String, name: String) {
    if let Ok(mut cache) = MOD_NAME_CACHE.lock() {
        cache.insert(mod_id, name);
    }
}

fn open_folder(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer").arg(path).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}

impl Render for LibraryDetailView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let back = div().flex().child(
            Button::new("back")
                .ghost()
                .icon(Icon::new(IconName::ArrowLeft))
                .label("Back")
                .on_click(cx.listener(|_, _, _window, cx| {
                    cx.emit(LibraryDetailEvent::Close);
                })),
        );

        let body: AnyElement = match &self.state {
            LoadState::Loading => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(Skeleton::new().w_1_3().h(px(28.0)).rounded_md())
                .child(Skeleton::new().w_2_3().h_4().rounded_md())
                .child(Skeleton::new().w_1_2().h_4().rounded_md())
                .child(Skeleton::new().w_full().h(px(120.0)).rounded_lg())
                .into_any_element(),
            LoadState::NotFound => div()
                .text_color(rgb(0xef4444))
                .child("Profile not found")
                .into_any_element(),
            LoadState::Failed(e) => div()
                .text_color(rgb(0xef4444))
                .child(format!("Failed: {e}"))
                .into_any_element(),
            LoadState::Loaded(profile) => {
                let bep_installed = profile.bepinex_installed == Some(true);
                let installing = self.bep_progress.is_some();

                let install_btn = (!bep_installed && !installing).then(|| {
                    Button::new("install-bepinex")
                        .primary()
                        .icon(Icon::new(AppIcon::Download))
                        .label("Install BepInEx")
                        .on_click(cx.listener(|this, _, _window, cx| this.install_bepinex(cx)))
                });

                let running = self.running_count;
                let stoppable = self.stoppable_count;
                let allow_multi = app_settings::get(cx).allow_multi_instance_launch;

                let launch_row = bep_installed.then(|| {
                    let mut row = div().flex().gap_2().items_center().flex_wrap();
                    if running == 0 {
                        row = row.child(
                            Button::new("launch")
                                .success()
                                .icon(Icon::new(IconName::Play))
                                .label("Launch")
                                .on_click(cx.listener(|this, _, _window, cx| this.launch(cx))),
                        );
                    } else {
                        let stop_label = if stoppable > 1 {
                            format!("Stop ({stoppable})")
                        } else {
                            "Stop".to_string()
                        };
                        let mut stop_btn = Button::new("stop")
                            .danger()
                            .icon(Icon::new(IconName::Close))
                            .label(stop_label);
                        if stoppable == 0 {
                            // Only UWP instances — can't stop those.
                            stop_btn = stop_btn.disabled(true);
                        } else {
                            stop_btn = stop_btn
                                .on_click(cx.listener(|this, _, _window, cx| this.stop(cx)));
                        }
                        row = row.child(stop_btn);
                        if allow_multi {
                            row = row.child(
                                Button::new("launch-another")
                                    .success()
                                    .icon(Icon::new(IconName::Play))
                                    .label("Launch another")
                                    .on_click(cx.listener(|this, _, _window, cx| this.launch(cx))),
                            );
                            row = row.child(div().text_sm().text_color(theme.text_muted).child(
                                format!(
                                    "{running} instance{} running",
                                    if running == 1 { "" } else { "s" }
                                ),
                            ));
                        }
                    }
                    row
                });

                let launch_err = self
                    .launch_error
                    .clone()
                    .map(|msg| div().text_sm().text_color(rgb(0xef4444)).child(msg));

                let progress_row = self.bep_progress.as_ref().map(|p| {
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.text_muted)
                                .child(format!("{} — {:.0}%", p.message, p.progress)),
                        )
                        .child(Progress::new("bep-progress").value(p.progress as f32))
                });

                let mod_names = self.mod_names.clone();
                let mods_section = (!profile.mods.is_empty()).then(|| {
                    let entries: Vec<AnyElement> = profile
                        .mods
                        .iter()
                        .enumerate()
                        .map(|(ix, m)| {
                            let display = mod_display_name(m, &mod_names);
                            let is_last = ix + 1 == profile.mods.len();
                            let mut row = div().flex().items_center().gap_3().px_3().py_2();
                            if !is_last {
                                row = row.border_b_1().border_color(theme.border);
                            }
                            row.child(
                                img(api::mod_thumbnail_url(&m.mod_id))
                                    .w(px(32.0))
                                    .h(px(32.0))
                                    .flex_none()
                                    .rounded_md()
                                    .object_fit(ObjectFit::Cover)
                                    .bg(theme.hover),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .truncate()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(display),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text_muted)
                                    .child(m.version.clone()),
                            )
                            .into_any_element()
                        })
                        .collect();
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(section_heading(&format!("Mods · {}", profile.mods.len())))
                        .child(
                            div()
                                .rounded_lg()
                                .bg(theme.sidebar_background)
                                .border_1()
                                .border_color(theme.border)
                                .children(entries),
                        )
                });

                let delete_row =
                    if self.confirming_delete {
                        div()
                            .flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .text_color(theme.text_muted)
                                    .child("Delete this profile?"),
                            )
                            .child(
                                Button::new("confirm-delete")
                                    .danger()
                                    .icon(Icon::new(IconName::Delete))
                                    .label("Delete")
                                    .on_click(
                                        cx.listener(|this, _, _window, cx| this.delete_profile(cx)),
                                    ),
                            )
                            .child(Button::new("cancel-delete").label("Cancel").on_click(
                                cx.listener(|this, _, _window, cx| {
                                    this.confirming_delete = false;
                                    cx.notify();
                                }),
                            ))
                    } else {
                        div().child(
                            Button::new("delete-profile")
                                .danger()
                                .icon(Icon::new(IconName::Delete))
                                .label("Delete Profile")
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.confirming_delete = true;
                                    cx.notify();
                                })),
                        )
                    };

                let manage_buttons = div()
                    .flex()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::new("rename-profile-action")
                            .label("Rename")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_rename_dialog(window, cx);
                            })),
                    )
                    .child(Button::new("edit-icon-action").label("Edit Icon").on_click(
                        cx.listener(|this, _, _window, cx| {
                            this.open_icon_dialog(cx);
                        }),
                    ))
                    .child(
                        Button::new("open-profile-folder-action")
                            .icon(Icon::new(IconName::FolderOpen))
                            .label("Open Folder")
                            .on_click(cx.listener(|this, _, _window, _cx| {
                                this.open_profile_folder();
                            })),
                    )
                    .child(
                        Button::new("export-profile-action")
                            .label("Export ZIP")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_export_dialog(window, cx);
                            })),
                    );

                let hero = div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    .rounded_lg()
                    .bg(theme.sidebar_background)
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap_4()
                            .child(profile_icon(profile, 80.0, &theme))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_2xl()
                                            .font_weight(FontWeight::BOLD)
                                            .truncate()
                                            .child(profile.name.clone()),
                                    )
                                    .child(
                                        div().text_sm().text_color(theme.text_muted).child(format!(
                                            "{} mods · {} played · Last launched {}",
                                            profile.mods.len(),
                                            format::play_time(profile.total_play_time),
                                            format::last_launched(profile.last_launched_at),
                                        )),
                                    )
                                    .children((!bep_installed).then(|| {
                                        div()
                                            .mt_1()
                                            .text_xs()
                                            .text_color(rgb(0xf59e0b))
                                            .child("⚠ BepInEx not installed")
                                    })),
                            )
                            .child(manage_buttons),
                    )
                    .children(progress_row)
                    .children(install_btn)
                    .children(launch_row)
                    .children(launch_err);

                let stats = div()
                    .grid()
                    .grid_cols(3)
                    .gap_3()
                    .child(stat_card(
                        "Created",
                        format::date_ms(profile.created_at),
                        &theme,
                    ))
                    .child(stat_card(
                        "Last launched",
                        format::last_launched(profile.last_launched_at),
                        &theme,
                    ))
                    .child(stat_card(
                        "Play time",
                        format::play_time(profile.total_play_time),
                        &theme,
                    ));

                let danger_zone = div()
                    .pt_2()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(delete_row);

                let has_log = self.log_panel.read(cx).has_content();

                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(hero)
                    .child(stats)
                    .children(mods_section)
                    .children(has_log.then(|| self.log_panel.clone().into_any_element()))
                    .child(danger_zone)
                    .into_any_element()
            }
        };

        div()
            .id("library-detail-page")
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
            .children(self.rename_dialog.clone().map(|input| {
                dialog_overlay(
                    input,
                    "Rename Profile",
                    "Save",
                    theme.clone(),
                    cx.listener(|this, _: &ClickEvent, _, cx| {
                        if let Some(input) = this.rename_dialog.clone() {
                            let name = input.read(cx).value().to_string();
                            this.submit_rename(name, cx);
                        }
                    }),
                    cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.rename_dialog = None;
                        cx.notify();
                    }),
                )
            }))
            .children(self.export_dialog.clone().map(|input| {
                dialog_overlay(
                    input,
                    "Export Profile ZIP",
                    "Export",
                    theme.clone(),
                    cx.listener(|this, _: &ClickEvent, _, cx| {
                        if let Some(input) = this.export_dialog.clone() {
                            let path = input.read(cx).value().to_string();
                            this.submit_export(path, cx);
                        }
                    }),
                    cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.export_dialog = None;
                        cx.notify();
                    }),
                )
            }))
            .children(
                self.icon_dialog
                    .as_ref()
                    .and_then(|s| match &self.state {
                        LoadState::Loaded(profile) => Some((s, profile.clone())),
                        _ => None,
                    })
                    .map(|(state, profile)| render_icon_dialog(state, &profile, theme.clone(), cx)),
            )
    }
}

fn section_heading(text: &str) -> impl IntoElement {
    div()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.to_string())
}

fn stat_card(label: &'static str, value: String, theme: &crate::theme::Theme) -> impl IntoElement {
    div()
        .rounded_lg()
        .bg(theme.sidebar_background)
        .border_1()
        .border_color(theme.border)
        .p_3()
        .child(div().text_xs().text_color(theme.text_muted).child(label))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(value))
}

/// The label to show for a mod row. Falls back to the on-disk filename when
/// the API name hasn't been resolved (or doesn't exist), and finally to the
/// raw BepInEx GUID if we don't even have a filename.
fn mod_display_name(m: &ProfileModEntry, names: &HashMap<String, String>) -> String {
    if let Some(name) = names.get(&m.mod_id) {
        return name.clone();
    }
    if let Some(file) = m.file.as_deref().filter(|s| !s.is_empty()) {
        return file.to_string();
    }
    m.mod_id.clone()
}

fn dialog_overlay(
    input: Entity<InputState>,
    title: &'static str,
    confirm: &'static str,
    theme: crate::theme::Theme,
    on_confirm: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
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
                .w(px(420.0))
                .p_5()
                .rounded_lg()
                .bg(theme.background)
                .border_1()
                .border_color(theme.border)
                .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                .child(Input::new(&input))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .justify_end()
                        .child(
                            Button::new("dialog-cancel")
                                .label("Cancel")
                                .on_click(on_cancel),
                        )
                        .child(
                            Button::new("dialog-confirm")
                                .primary()
                                .label(confirm)
                                .on_click(on_confirm),
                        ),
                ),
        )
}
