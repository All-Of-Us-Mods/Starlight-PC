//! Profile detail page. Composes the launch / metadata controls with the
//! reusable [`LogPanel`] (in `ui/log_panel`) and the icon picker dialog (in
//! `icon_dialog`). Long-running work — disk reads, API lookups, launch —
//! always happens on the background executor; this module only orchestrates.

mod icon_dialog;
mod installation_picker;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use log::warn;
use rust_i18n::t;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::backend::api;
use crate::backend::events::{self, BackendEvent};
use crate::backend::services::bepinex_service::{BepInExProgress, BepInExTargetType};
use crate::backend::services::launch_service;
use crate::backend::services::mod_install_service;
use crate::backend::services::profile_service::{
    self, BepInExStatus, ProfileEntry, ProfileModEntry, ZipOp,
};
#[cfg(windows)]
use crate::backend::services::profile_shortcut_service;
use crate::backend::state::game_runtime;
use crate::backend::state::mod_catalog_cache;
use crate::settings as app_settings;
use crate::ui::file_drop;
use crate::ui::format;
use crate::ui::icon::AppIcon;
use crate::ui::log_panel::LogPanel;
use crate::ui::profile_icon::profile_icon;
use crate::views::{empty_state, page_root};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::avatar::Avatar;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_kit::component::dialog::{DialogAction, DialogButtonProps, DialogClose, DialogFooter};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::progress::Progress;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{Disableable, Icon, IconName, Sizable, WindowExt};

use icon_dialog::IconDialogState;

#[derive(Clone, Debug)]
pub enum LibraryDetailEvent {
    Close,
    /// The user clicked "Install Mods" — navigate to the Explore page.
    OpenExplore,
}

impl EventEmitter<LibraryDetailEvent> for LibraryDetailView {}

pub struct LibraryDetailView {
    pub(super) profile_id: String,
    pub(super) state: LoadState,
    bep_progress: Option<BepInExProgress>,
    launch_error: Option<String>,
    /// Success/info message (e.g. "Exported profile to …"); rendered non-red.
    notice: Option<String>,
    rename_dialog: Option<Entity<InputState>>,
    /// 0–100 while an export is running; `None` otherwise.
    export_progress: Option<f64>,
    pub(super) icon_dialog: Option<IconDialogState>,
    /// This profile's running instances plus launches still being prepared.
    running_count: usize,
    log_panel: Entity<LogPanel>,
    /// API-resolved display names per mod_id, populated lazily after load.
    mod_names: HashMap<String, String>,
    /// Latest catalog release per mod_id, populated alongside display names.
    mod_latest_versions: HashMap<String, String>,
    /// Mod ids currently being updated. Kept per row so unrelated controls
    /// remain available while one download is in flight.
    updating_mods: HashSet<String>,
    /// Serializes separately requested updates for this profile: two batches
    /// can share a dependency, and would download over the same file and roll
    /// back each other's manifest entries.
    update_lock: Arc<Mutex<()>>,
}

pub(super) enum LoadState {
    Loading,
    Loaded(Box<ProfileEntry>),
    NotFound,
    Failed(String),
}

impl LibraryDetailView {
    pub fn new(profile_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let log_panel = cx.new(|cx| LogPanel::new(window, cx));
        // `render` reads `has_content` even while the panel isn't shown, and
        // a hidden panel's notify doesn't reach this (cached) view on its own.
        cx.observe(&log_panel, |_, _, cx| cx.notify()).detach();

        let view = Self {
            profile_id: profile_id.clone(),
            state: LoadState::Loading,
            bep_progress: None,
            launch_error: None,
            notice: None,
            rename_dialog: None,
            export_progress: None,
            icon_dialog: None,
            running_count: game_runtime::current_state().profile_count(&profile_id),
            log_panel,
            mod_names: mod_catalog_cache::cached_names(),
            mod_latest_versions: mod_catalog_cache::cached_latest_versions(),
            updating_mods: HashSet::new(),
            update_lock: Arc::new(Mutex::new(())),
        };

        view.spawn_load(cx);
        cx.observe_global::<app_settings::SettingsGlobal>(|_, cx| cx.notify())
            .detach();

        // Reload when the window regains focus, so a DLL dropped into
        // BepInEx/plugins via the file manager shows up without leaving and
        // reopening the page.
        cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.spawn_load(cx);
            }
        })
        .detach();

        // Backend events for *this* profile.
        events::listen(cx, |this, event, cx| match event {
            BackendEvent::BepInExProgress(p)
                if matches!(p.target_type, BepInExTargetType::Profile)
                    && p.target_id == this.profile_id =>
            {
                let done = matches!(p.stage.as_str(), "complete" | "failed");
                if p.stage == "failed" {
                    this.launch_error = Some(p.message.clone());
                }
                this.bep_progress = if done { None } else { Some(p) };
                cx.notify();
                if done {
                    this.spawn_load(cx);
                }
            }
            BackendEvent::GameStateChanged(payload) => {
                this.running_count = payload.profile_count(&this.profile_id);
                cx.notify();
                // Game state change ≈ new log content / mod changes.
                this.refresh_disk_state(cx);
            }
            BackendEvent::ProfileChanged(id) if id == this.profile_id => this.spawn_load(cx),
            BackendEvent::ZipProgress(p) if matches!(&p.op, ZipOp::Export(id) if *id == this.profile_id) =>
            {
                this.export_progress = Some(p.progress);
                cx.notify();
            }
            _ => {}
        });

        view
    }

    /// Title shown in the app title bar — the profile name once loaded.
    pub fn title(&self) -> SharedString {
        match &self.state {
            LoadState::Loaded(profile) => profile.name.clone().into(),
            _ => t!("profile.fallback_title").into(),
        }
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
                    Ok(Some(p)) => LoadState::Loaded(Box::new(p)),
                    Ok(None) => LoadState::NotFound,
                    Err(e) => LoadState::Failed(e.to_string()),
                };
                cx.notify();
                this.refresh_disk_state(cx);
                this.fetch_mod_catalog_data(cx);
            });
        })
        .detach();
    }

    /// Resolve display names and latest releases for installed catalog mods.
    /// Successful lookups are shared across detail-page instances for the app
    /// session, so returning to a profile does not repeat network requests.
    fn fetch_mod_catalog_data(&self, cx: &mut Context<Self>) {
        let LoadState::Loaded(profile) = &self.state else {
            return;
        };
        let mod_ids: Vec<String> = profile.mods.iter().map(|m| m.mod_id.clone()).collect();
        if mod_ids.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            let tasks: Vec<_> = mod_ids
                .into_iter()
                .map(|mod_id| {
                    let id_for_fetch = mod_id.clone();
                    let task = cx.background_executor().spawn(async move {
                        let name = mod_catalog_cache::fetch(&id_for_fetch).map(|m| m.name);
                        let latest = name
                            .as_ref()
                            .and_then(|_| mod_catalog_cache::fetch_latest_version(&id_for_fetch));
                        (name, latest)
                    });
                    (mod_id, task)
                })
                .collect();
            for (mod_id, task) in tasks {
                let (name, latest) = task.await;
                if name.is_some() || latest.is_some() {
                    let _ = this.update(cx, |this, cx| {
                        if let Some(name) = name {
                            this.mod_names.insert(mod_id.clone(), name);
                        }
                        if let Some(latest) = latest {
                            this.mod_latest_versions.insert(mod_id, latest);
                        }
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
        self.launch_error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::install_bepinex_for_profile(&id) })
                .await;
            if let Err(error) = result {
                warn!("install_bepinex_for_profile failed: {error}");
                let _ = this.update(cx, |this, cx| {
                    this.bep_progress = None;
                    this.launch_error = Some(error.to_string());
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn toggle_mod(&mut self, file: String, enabled: bool, cx: &mut Context<Self>) {
        // Optimistic UI update; reverted by a reload if the on-disk op fails.
        if let LoadState::Loaded(profile) = &mut self.state {
            let catalog = profile
                .mods
                .iter_mut()
                .filter(|m| m.file.as_ref() == Some(&file));
            for entry in catalog {
                entry.enabled = enabled;
            }
            for entry in profile.custom_mods.iter_mut().filter(|m| m.file == file) {
                entry.enabled = enabled;
            }
        }
        cx.notify();

        let profile_id = self.profile_id.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(
                    async move { profile_service::set_plugin_enabled(&profile_id, &file, enabled) },
                )
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    warn!("toggle mod failed: {e}");
                    this.launch_error = Some(t!("profile.toggle_failed", error = e).to_string());
                    this.spawn_load(cx);
                }
            });
        })
        .detach();
    }

    /// Update one or more outdated catalog mods as a rollback-safe batch.
    /// Required dependencies are installed first; optional branches remain
    /// opt-in via the normal install flow.
    fn update_mods(&mut self, mut updates: Vec<(String, String)>, cx: &mut Context<Self>) {
        updates.retain(|(mod_id, _)| !self.updating_mods.contains(mod_id));
        if updates.is_empty() {
            return;
        }
        self.updating_mods
            .extend(updates.iter().map(|(mod_id, _)| mod_id.clone()));
        self.notice = None;
        self.launch_error = None;
        cx.notify();

        let profile_id = self.profile_id.clone();
        let updating_ids: HashSet<String> =
            updates.iter().map(|(mod_id, _)| mod_id.clone()).collect();
        let updated_count = updates.len();
        let update_lock = self.update_lock.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let _update_guard = update_lock.lock().map_err(|_| {
                        crate::backend::error::AppError::state("Mod update lock was poisoned")
                    })?;
                    mod_install_service::update_profile_mods(&profile_id, updates)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                for mod_id in &updating_ids {
                    this.updating_mods.remove(mod_id);
                }
                match result {
                    Ok(()) => {
                        this.notice = Some(if updated_count == 1 {
                            t!("profile.updated_one_mod").to_string()
                        } else {
                            t!("profile.updated_mods", count = updated_count).to_string()
                        });
                    }
                    Err(e) => {
                        warn!("update mods failed: {e}");
                        this.launch_error =
                            Some(t!("profile.update_mods_failed", error = e).to_string());
                    }
                }
                cx.notify();
                this.spawn_load(cx);
            });
        })
        .detach();
    }

    /// Ask before removing a mod from the profile — deleting its files can't
    /// be undone from here.
    fn confirm_delete_mod(
        &mut self,
        target: ModRef,
        display: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity();
        window.open_alert_dialog(cx, move |alert, _window, cx| {
            let view = view.clone();
            let target = target.clone();
            alert
                .icon(Icon::new(IconName::TriangleAlert).text_color(cx.theme().danger))
                .title(t!("profile.remove_mod_title"))
                .description(t!("profile.remove_mod_desc", name = display).to_string())
                .button_props(
                    DialogButtonProps::default()
                        .ok_variant(ButtonVariant::Danger)
                        .ok_text(t!("profile.remove"))
                        .cancel_text(t!("common.cancel"))
                        .show_cancel(true),
                )
                .on_ok(move |_, _window, cx| {
                    let target = target.clone();
                    view.update(cx, |this, cx| this.delete_mod(target, cx));
                    true
                })
        });
    }

    fn delete_mod(&mut self, target: ModRef, cx: &mut Context<Self>) {
        // Optimistically drop the row; the reload afterwards confirms it (or
        // brings it back if the on-disk op failed).
        if let LoadState::Loaded(profile) = &mut self.state {
            match &target {
                ModRef::Catalog(mod_id) => profile.mods.retain(|m| &m.mod_id != mod_id),
                ModRef::Custom(file) => profile.custom_mods.retain(|m| &m.file != file),
            }
        }
        cx.notify();

        let profile_id = self.profile_id.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    match target {
                        ModRef::Catalog(mod_id) => {
                            profile_service::uninstall_mod_from_profile(&profile_id, &mod_id)
                        }
                        ModRef::Custom(file) => {
                            profile_service::remove_custom_mod(&profile_id, &file)
                        }
                    }
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    warn!("delete mod failed: {e}");
                    this.launch_error =
                        Some(t!("profile.remove_mod_failed", error = e).to_string());
                }
                this.spawn_load(cx);
            });
        })
        .detach();
    }

    fn open_profile_folder(&self, cx: &App) {
        if let LoadState::Loaded(profile) = &self.state {
            cx.open_with_system(Path::new(&profile.path));
        }
    }

    fn launch(&mut self, cx: &mut Context<Self>) {
        let LoadState::Loaded(profile) = &self.state else {
            return;
        };
        let profile = profile.as_ref().clone();
        self.launch_error = None;
        cx.notify();
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
        cx.notify();
        cx.background_executor()
            .spawn(async move { game_runtime::stop_profile_instances(&id) })
            .detach();
    }

    /// Deleting a profile wipes its mods and logs, so it goes through a
    /// confirmation first.
    fn confirm_delete_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = match &self.state {
            LoadState::Loaded(profile) => profile.name.clone(),
            _ => t!("profile.this_profile").to_string(),
        };
        let view = cx.entity();
        window.open_alert_dialog(cx, move |alert, _window, cx| {
            let view = view.clone();
            alert
                .icon(Icon::new(IconName::TriangleAlert).text_color(cx.theme().danger))
                .title(t!("profile.delete_title"))
                .description(t!("profile.delete_desc", name = name).to_string())
                .button_props(
                    DialogButtonProps::default()
                        .ok_variant(ButtonVariant::Danger)
                        .ok_text(t!("common.delete"))
                        .cancel_text(t!("common.cancel"))
                        .show_cancel(true),
                )
                .on_ok(move |_, _window, cx| {
                    view.update(cx, |this, cx| this.delete_profile(cx));
                    true
                })
        });
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
                    this.launch_error = Some(t!("profile.delete_failed", error = e).to_string());
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
            let mut s =
                InputState::new(window, cx).placeholder(t!("library.profile_name").to_string());
            s.set_value(name, window, cx);
            s
        });
        state.read(cx).focus_handle(cx).focus(window, cx);
        cx.subscribe_in(
            &state,
            window,
            |this, state, event: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    let name = state.read(cx).value().to_string();
                    this.submit_rename(name, cx);
                    // An empty name is rejected and leaves the state in place;
                    // only a submit that took closes the dialog.
                    if this.rename_dialog.is_none() {
                        window.close_dialog(cx);
                    }
                }
            },
        )
        .detach();
        // Kept here so the dialog builder can read the typed name back out.
        self.rename_dialog = Some(state.clone());
        let view = cx.entity();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let input = state.clone();
            let on_ok = view.clone();
            let on_close = view.clone();
            dialog
                .title(t!("profile.rename_title"))
                .w(px(420.0))
                .child(Input::new(&input))
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new()
                                .child(Button::new("rename-cancel").label(t!("common.cancel"))),
                        )
                        .child(
                            DialogAction::new().child(
                                Button::new("rename-confirm")
                                    .primary()
                                    .label(t!("common.save")),
                            ),
                        ),
                )
                .on_ok(move |_, _window, cx| {
                    on_ok.update(cx, |this, cx| {
                        if let Some(input) = this.rename_dialog.clone() {
                            let name = input.read(cx).value().to_string();
                            this.submit_rename(name, cx);
                        }
                        // Stays open while the name is still empty.
                        this.rename_dialog.is_none()
                    })
                })
                .on_close(move |_, _window, cx| {
                    on_close.update(cx, |this, cx| {
                        this.rename_dialog = None;
                        cx.notify();
                    });
                })
        });
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
                    this.launch_error = Some(t!("profile.rename_failed", error = e).to_string());
                    cx.notify();
                }
                this.spawn_load(cx);
            });
        })
        .detach();
    }

    /// Open the native file picker and copy the chosen plugin .dll(s) into
    /// this profile's `BepInEx/plugins`. The reload afterwards surfaces them
    /// as custom mod entries.
    fn add_custom_mods(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(t!("profile.add_dll_prompt").to_string().into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let _ = this.update(cx, |this, cx| this.add_mod_paths(paths, cx));
        })
        .detach();
    }

    /// Plugin .dlls dropped onto this profile's page go straight in — same
    /// path as the picker above.
    fn on_drop(&mut self, paths: &[PathBuf], cx: &mut Context<Self>) {
        let dropped = file_drop::DroppedFiles::classify(paths);
        if dropped.plugins.is_empty() {
            self.launch_error = Some(if dropped.archives.is_empty() {
                t!("profile.drop_dll").to_string()
            } else {
                // Importing an archive here would create a second profile,
                // which is not what dropping it on this page suggests.
                t!("profile.drop_zip").to_string()
            });
            self.notice = None;
            cx.notify();
            return;
        }
        self.add_mod_paths(dropped.plugins.into_iter().map(PathBuf::from).collect(), cx);
    }

    fn add_mod_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if paths.is_empty() {
            return;
        }
        let profile_id = self.profile_id.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let mut added = Vec::new();
                    for path in paths {
                        added.push(profile_service::import_mod_to_profile(
                            &profile_id,
                            &path.to_string_lossy(),
                        )?);
                    }
                    Ok::<_, crate::backend::error::AppError>(added)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(added) => {
                        this.notice =
                            Some(t!("profile.added", names = added.join(", ")).to_string());
                        this.launch_error = None;
                    }
                    Err(e) => {
                        this.launch_error =
                            Some(t!("library.add_mod_failed", error = e).to_string());
                        this.notice = None;
                    }
                }
                cx.notify();
                this.spawn_load(cx);
            });
        })
        .detach();
    }

    /// Write a `.url` shortcut for this profile onto the desktop. It opens the
    /// app via the `starlight://` scheme, which auto-launches the profile.
    #[cfg(windows)]
    fn create_desktop_shortcut(&mut self, cx: &mut Context<Self>) {
        let id = self.profile_id.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_shortcut_service::create_desktop_shortcut(&id) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(path) => {
                        this.notice = Some(t!("profile.shortcut_created", path = path).to_string());
                        this.launch_error = None;
                    }
                    Err(e) => {
                        this.launch_error =
                            Some(t!("profile.shortcut_failed", error = e).to_string());
                        this.notice = None;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Open the native save dialog and export this profile to the chosen .zip.
    fn export_profile(&mut self, cx: &mut Context<Self>) {
        let LoadState::Loaded(profile) = &self.state else {
            return;
        };
        let id = self.profile_id.clone();
        let suggested = format!("{}.zip", profile.name);
        let receiver = cx.prompt_for_new_path(&default_export_dir(), Some(&suggested));
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = receiver.await else {
                return;
            };
            let dest = path.to_string_lossy().into_owned();
            let dest_for_task = dest.clone();
            let _ = this.update(cx, |this, cx| {
                this.export_progress = Some(0.0);
                this.notice = None;
                this.launch_error = None;
                cx.notify();
            });
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::export_profile_zip(&id, &dest_for_task) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.export_progress = None;
                match result {
                    Ok(()) => this.notice = Some(t!("profile.exported", dest = dest).to_string()),
                    Err(e) => {
                        this.launch_error = Some(t!("profile.export_failed", error = e).to_string())
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}

/// Starting directory for the export save dialog (user home, else cwd).
fn default_export_dir() -> std::path::PathBuf {
    std::env::home_dir().unwrap_or_else(|| ".".into())
}

impl Render for LibraryDetailView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

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
            LoadState::NotFound => {
                Alert::error("profile-not-found", t!("profile.not_found").to_string())
                    .into_any_element()
            }
            LoadState::Failed(e) => Alert::error(
                "profile-load-failed",
                t!("common.failed", error = e).to_string(),
            )
            .into_any_element(),
            LoadState::Loaded(profile) => {
                // Each section is built by a helper so its (large) element
                // temporaries live in that helper's stack frame, not all in
                // render()'s — one flat frame overflows the stack in debug
                // builds on Windows.
                let profile = profile.clone();
                let hero = self.render_hero(&profile, &theme, cx);
                let mods_section = self.render_mods_section(&profile, &theme, cx);
                let danger_zone = self.render_danger_zone(&theme, cx);
                let has_log = self.log_panel.read(cx).has_content();

                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(hero)
                    .child(mods_section)
                    .children(has_log.then(|| self.log_panel.clone().into_any_element()))
                    .child(danger_zone)
                    .into_any_element()
            }
        };

        page_root("library-detail-page", &theme)
            .gap_4()
            // Dropping plugin .dlls anywhere on the page adds them to this profile.
            .drag_over::<ExternalPaths>({
                let hover = theme.accent;
                move |style, _, _, _| style.bg(hover)
            })
            .on_drop(cx.listener(|this, dropped: &ExternalPaths, _window, cx| {
                this.on_drop(dropped.paths(), cx);
            }))
            // Chained after the drop handlers: the scrollbar wrapper is no
            // longer a stateful element, so `on_drop` has to come first.
            .overflow_y_scrollbar()
            .children(self.export_progress.map(|p| {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div().text_sm().text_color(theme.muted_foreground).child(
                            t!("profile.exporting", percent = format!("{p:.0}")).to_string(),
                        ),
                    )
                    .child(Progress::new("export-progress").value(p as f32))
            }))
            .child(body)
    }
}

impl LibraryDetailView {
    /// Top-right hero action: Install BepInEx until it's present, then
    /// Launch / Stop. `None` while an install is in flight (the progress row
    /// covers that state).
    fn render_primary_controls(
        &self,
        // `Some` when BepInEx must be (re)installed before launching.
        install_label: Option<Cow<'static, str>>,
        installing: bool,
        installation_available: bool,
        theme: &gpui_kit::component::Theme,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        // Pending launches count as running: Stop cancels them too.
        let running = self.running_count;
        let allow_multi = app_settings::get(cx).allow_multi_instance_launch;

        if installing {
            None
        } else if let Some(label) = install_label {
            Some(
                Button::new("install-bepinex")
                    .primary()
                    .large()
                    .icon(Icon::new(AppIcon::Download))
                    .label(label)
                    .disabled(!installation_available)
                    .on_click(cx.listener(|this, _, _window, cx| this.install_bepinex(cx)))
                    .into_any_element(),
            )
        } else {
            let mut row = div()
                .flex()
                .gap_2()
                .items_center()
                .justify_end()
                .flex_nowrap();
            if running == 0 {
                row = row.child(
                    Button::new("launch")
                        .success()
                        .large()
                        .icon(Icon::new(IconName::Play))
                        .label(t!("lobbies.launch"))
                        .disabled(!installation_available)
                        .on_click(cx.listener(|this, _, _window, cx| this.launch(cx))),
                );
            } else {
                let stop_label = if running > 1 {
                    t!("titlebar.stop_count", count = running).to_string()
                } else {
                    t!("common.stop").to_string()
                };
                let stop_btn = Button::new("stop")
                    .danger()
                    .large()
                    .icon(Icon::new(IconName::Close))
                    .label(stop_label)
                    .on_click(cx.listener(|this, _, _window, cx| this.stop(cx)));
                if allow_multi {
                    row = row.child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(t!("profile.instances_running", count = running).to_string()),
                    );
                    row = row.child(
                        Button::new("launch-another")
                            .success()
                            .large()
                            .icon(Icon::new(IconName::Play))
                            .label(t!("profile.launch_another"))
                            .disabled(!installation_available)
                            .on_click(cx.listener(|this, _, _window, cx| this.launch(cx))),
                    );
                }
                row = row.child(stop_btn);
            }
            Some(row.into_any_element())
        }
    }

    /// Quiet toolbar of secondary profile actions shown at the bottom of the
    /// hero card. Rename and icon editing live inline on the hero itself,
    /// revealed on hover.
    fn render_manage_buttons(&self, cx: &mut Context<Self>) -> AnyElement {
        let manage_buttons = div()
            .flex()
            .gap_1()
            .flex_wrap()
            .child(
                Button::new("open-profile-folder-action")
                    .ghost()
                    .small()
                    .icon(Icon::new(IconName::FolderOpen))
                    .label(t!("profile.open_folder"))
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.open_profile_folder(cx);
                    })),
            )
            .child(
                Button::new("export-profile-action")
                    .ghost()
                    .small()
                    .icon(Icon::new(IconName::HardDrive))
                    .label(t!("profile.export_zip"))
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.export_profile(cx);
                    })),
            );

        #[cfg(windows)]
        let manage_buttons = manage_buttons.child(
            Button::new("create-shortcut-action")
                .ghost()
                .small()
                .icon(Icon::new(IconName::ExternalLink))
                .label(t!("profile.desktop_shortcut"))
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.create_desktop_shortcut(cx);
                })),
        );

        manage_buttons.into_any_element()
    }

    fn render_hero(
        &self,
        profile: &ProfileEntry,
        theme: &gpui_kit::component::Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let settings = app_settings::get(cx);
        let status = profile.bepinex_status(settings);
        let missing_installation = status == BepInExStatus::MissingInstallation;
        let (install_label, warning) = match status {
            BepInExStatus::NotInstalled => (
                Some(t!("profile.install_bepinex")),
                Some(t!("profile.bepinex_not_installed").to_string()),
            ),
            BepInExStatus::Incompatible => (
                Some(t!("profile.reinstall_bepinex")),
                profile.installation(settings).ok().map(|game| {
                    t!(
                        "profile.bepinex_incompatible",
                        platform = game.game_platform.display_name()
                    )
                    .to_string()
                }),
            ),
            BepInExStatus::Ready | BepInExStatus::MissingInstallation => (None, None),
        };
        let installing = self.bep_progress.is_some();
        let primary_controls = self.render_primary_controls(
            install_label,
            installing,
            !missing_installation,
            theme,
            cx,
        );
        let manage_buttons = self.render_manage_buttons(cx);

        let launch_err = self.launch_error.clone().map(|msg| {
            Alert::error("profile-launch-error", msg)
                .small()
                .on_close(cx.listener(|this, _, _window, cx| {
                    this.launch_error = None;
                    cx.notify();
                }))
        });

        let notice = self.notice.clone().map(|msg| {
            Alert::success("profile-notice", msg)
                .small()
                .on_close(cx.listener(|this, _, _window, cx| {
                    this.notice = None;
                    cx.notify();
                }))
        });

        let progress_row = self.bep_progress.as_ref().map(|p| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!("{} — {:.0}%", p.message, p.progress)),
                )
                .child(Progress::new("bep-progress").value(p.progress as f32))
        });

        let title_col = div()
            .flex()
            .flex_col()
            .gap_1()
            .flex_1()
            .min_w_0()
            .child(
                div()
                    .id("profile-name-area")
                    .group("profile-name-area")
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_rename_dialog(window, cx);
                    }))
                    .child(
                        div()
                            .min_w_0()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .truncate()
                            .child(profile.name.clone()),
                    )
                    .child(
                        // Affordance only — the whole name area is the click
                        // target. Revealed on hover.
                        div()
                            .flex_none()
                            .invisible()
                            .group_hover("profile-name-area", |s| s.visible())
                            .child(
                                Icon::new(AppIcon::Pencil)
                                    .small()
                                    .text_color(theme.muted_foreground),
                            ),
                    ),
            )
            .child(
                div().text_sm().text_color(theme.muted_foreground).child(
                    t!(
                        "profile.played",
                        time = format::play_time(profile.total_play_time)
                    )
                    .to_string(),
                ),
            )
            .children(warning.map(|warning| {
                div()
                    .mt_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_xs()
                    .text_color(theme.warning)
                    .child(Icon::new(IconName::TriangleAlert).xsmall())
                    .child(warning)
            }));

        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .rounded_lg()
            .bg(theme.secondary)
            .border_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .flex_wrap()
                    .child(
                        div()
                            .group("profile-icon-area")
                            .relative()
                            .flex_none()
                            .child(profile_icon(profile, 80.0))
                            .child(
                                // Revealed on hover; hidden, it takes no clicks.
                                div()
                                    .id("profile-icon-edit")
                                    .absolute()
                                    .inset_0()
                                    .rounded_md()
                                    .bg(black().opacity(0.55))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .invisible()
                                    .group_hover("profile-icon-area", |s| s.visible())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_icon_dialog(window, cx);
                                    }))
                                    // White reads on the dark scrim
                                    // regardless of image or theme.
                                    .child(Icon::new(AppIcon::Pencil).text_color(white())),
                            ),
                    )
                    .child(title_col)
                    .children(primary_controls.map(|c| div().flex_none().child(c))),
            )
            .when(
                app_settings::get(cx).show_installation_controls || missing_installation,
                |hero| {
                    hero.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(t!("profile.launch_using").to_string())
                            .child(self.installation_picker(profile, cx)),
                    )
                },
            )
            .children(progress_row)
            .children(missing_installation.then(|| {
                Alert::error(
                    "profile-missing-installation",
                    t!("profile.missing_install").to_string(),
                )
                .small()
            }))
            .children(launch_err)
            .children(notice)
            // Secondary profile actions live in a quiet toolbar under a
            // divider so the launch action above stays the focal point.
            .child(
                div()
                    .pt_3()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(manage_buttons),
            )
            .into_any_element()
    }

    fn render_mods_section(
        &self,
        profile: &ProfileEntry,
        theme: &gpui_kit::component::Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mod_names = self.mod_names.clone();
        let latest_versions = self.mod_latest_versions.clone();
        let updates: Vec<(String, String)> = profile
            .mods
            .iter()
            .filter_map(|installed| {
                if !installed.enabled {
                    return None;
                }
                latest_versions.get(&installed.mod_id).and_then(|latest| {
                    mod_catalog_cache::is_version_outdated(&installed.version, latest)
                        .then(|| (installed.mod_id.clone(), latest.clone()))
                })
            })
            .collect();
        let outdated_count = updates.len();
        let updating_all = !self.updating_mods.is_empty();
        {
            let rows: Vec<ModRow> = profile
                .mods
                .iter()
                .map(|m| ModRow {
                    target: ModRef::Catalog(m.mod_id.clone()),
                    name: mod_display_name(m, &mod_names),
                    file_note: None,
                    version: m.version.clone(),
                    thumbnail: Some(api::mod_thumbnail_url(&m.mod_id)),
                    file: m.file.clone(),
                    enabled: m.enabled,
                    update: latest_versions
                        .get(&m.mod_id)
                        .filter(|latest| {
                            m.enabled && mod_catalog_cache::is_version_outdated(&m.version, latest)
                        })
                        .map(|latest| (m.mod_id.clone(), latest.clone())),
                    updating: self.updating_mods.contains(&m.mod_id),
                })
                .chain(profile.custom_mods.iter().map(|m| {
                    ModRow {
                        target: ModRef::Custom(m.file.clone()),
                        name: m.display_name().to_string(),
                        // The metadata name can be shared, e.g. by a native
                        // library's helper DLLs.
                        file_note: m.name.is_some().then(|| m.file_name().to_string()),
                        version: m
                            .version
                            .clone()
                            .unwrap_or_else(|| t!("profile.custom_mod").to_string()),
                        thumbnail: None,
                        file: Some(m.file.clone()),
                        enabled: m.enabled,
                        update: None,
                        updating: false,
                    }
                }))
                .collect();
            let mod_count = rows.len();
            let entries: Vec<AnyElement> = rows
                .into_iter()
                .enumerate()
                .map(|(ix, m)| {
                    let is_last = ix + 1 == mod_count;
                    let name_color = if m.enabled {
                        theme.foreground
                    } else {
                        theme.muted_foreground
                    };
                    let updating = m.updating;
                    // Custom mods fall back to the placeholder icon.
                    let thumbnail = Avatar::new()
                        .with_size(px(32.0))
                        .rounded_md()
                        .placeholder(Icon::new(IconName::File))
                        .when_some(m.thumbnail, |this, src| this.src(src));
                    let version: AnyElement = match m.update.as_ref() {
                        Some((_, latest)) => div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .flex_none()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(m.version)
                            .child(Icon::new(IconName::ArrowRight).xsmall())
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(name_color)
                                    .child(latest.clone()),
                            )
                            .into_any_element(),
                        None => div()
                            .flex_none()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(m.version)
                            .into_any_element(),
                    };
                    let group = SharedString::from(format!("mod-row-{ix}"));
                    let mut row = div()
                        .group(group.clone())
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_3()
                        .py_2()
                        .hover({
                            let hover_bg = theme.accent;
                            move |s| s.bg(hover_bg)
                        });
                    if !is_last {
                        row = row.border_b_1().border_color(theme.border);
                    }
                    let (target, name) = (m.target, m.name.clone());
                    row.child(thumbnail)
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(name_color)
                                        .child(m.name),
                                )
                                .children(m.file_note.map(|file| {
                                    div()
                                        .invisible()
                                        .group_hover(group, |s| s.visible())
                                        .min_w_0()
                                        .flex_1()
                                        .truncate()
                                        .text_size(px(10.0))
                                        .text_color(theme.muted_foreground)
                                        .child(file)
                                })),
                        )
                        .child(version)
                        .children(m.update.map(|update| {
                            Button::new(SharedString::from(format!("mod-update-{ix}")))
                                .ghost()
                                .small()
                                .icon(Icon::new(AppIcon::Download))
                                .label(if updating {
                                    t!("profile.updating_mod")
                                } else {
                                    t!("profile.update_mod")
                                })
                                .loading(updating)
                                .on_click(cx.listener(move |this, _, _window, cx| {
                                    this.update_mods(vec![update.clone()], cx)
                                }))
                        }))
                        // Mods imported without a known filename can't be toggled on disk.
                        .children(m.file.map(|file| {
                            Switch::new(SharedString::from(format!("mod-toggle-{ix}")))
                                .checked(m.enabled)
                                .disabled(updating)
                                .on_click(cx.listener(move |this, checked: &bool, _window, cx| {
                                    this.toggle_mod(file.clone(), *checked, cx)
                                }))
                        }))
                        .child(
                            Button::new(SharedString::from(format!("mod-delete-{ix}")))
                                .ghost()
                                .icon(Icon::new(IconName::Delete))
                                .tooltip(t!("profile.remove_mod_title"))
                                .disabled(updating)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.confirm_delete_mod(
                                        target.clone(),
                                        name.clone(),
                                        window,
                                        cx,
                                    )
                                })),
                        )
                        .into_any_element()
                })
                .collect();
            let list: AnyElement = if entries.is_empty() {
                empty_state(t!("profile.no_mods")).into_any_element()
            } else {
                div().children(entries).into_any_element()
            };
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(section_heading(
                            t!("profile.mods_heading", count = mod_count).as_ref(),
                        ))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .children((outdated_count > 0).then(|| {
                                    let updates = updates.clone();
                                    Button::new("update-all-mods")
                                        .primary()
                                        .icon(Icon::new(AppIcon::Download))
                                        .label(if updating_all {
                                            t!("profile.updating_all_mods").to_string()
                                        } else {
                                            t!("profile.update_all_mods", count = outdated_count,)
                                                .to_string()
                                        })
                                        .loading(updating_all)
                                        .on_click(cx.listener(move |this, _, _window, cx| {
                                            this.update_mods(updates.clone(), cx)
                                        }))
                                }))
                                .child(
                                    Button::new("install-mods")
                                        .icon(Icon::new(AppIcon::Compass))
                                        .label(t!("profile.install_mods"))
                                        .on_click(cx.listener(|_, _, _window, cx| {
                                            cx.emit(LibraryDetailEvent::OpenExplore)
                                        })),
                                )
                                .child(
                                    Button::new("add-custom-mod")
                                        .icon(Icon::new(IconName::Plus))
                                        .label(t!("profile.add_dll"))
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            this.add_custom_mods(cx)
                                        })),
                                ),
                        ),
                )
                .child(
                    div()
                        .rounded_lg()
                        .bg(theme.secondary)
                        .border_1()
                        .border_color(theme.border)
                        // Clip row hover backgrounds to the rounded corners.
                        .overflow_hidden()
                        .child(list),
                )
                .into_any_element()
        }
    }

    fn render_danger_zone(
        &self,
        theme: &gpui_kit::component::Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let delete_controls = Button::new("delete-profile")
            .danger()
            .outline()
            .icon(Icon::new(IconName::Delete))
            .label(t!("profile.delete_profile"))
            .on_click(cx.listener(|this, _, window, cx| {
                this.confirm_delete_profile(window, cx);
            }));

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(section_heading(t!("profile.danger_zone").as_ref()))
            .child(
                div()
                    .rounded_lg()
                    .bg(theme.secondary)
                    .border_1()
                    .border_color(theme.danger.alpha(0.45))
                    .px_4()
                    .py_3()
                    .flex()
                    .items_center()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(t!("profile.delete_this_profile").to_string()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(t!("profile.delete_this_profile_desc").to_string()),
                            ),
                    )
                    .child(delete_controls),
            )
            .into_any_element()
    }
}

fn section_heading(text: &str) -> impl IntoElement {
    div()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.to_string())
}

/// What a row's delete button removes: a catalog mod by id, a custom mod by
/// its plugin file.
#[derive(Clone)]
enum ModRef {
    Catalog(String),
    Custom(String),
}

/// One row of the mods list; catalog and custom mods render alike.
struct ModRow {
    target: ModRef,
    name: String,
    /// The plugin file, shown beside a name that isn't derived from it.
    file_note: Option<String>,
    version: String,
    thumbnail: Option<String>,
    /// Without one the mod can't be toggled on disk.
    file: Option<String>,
    enabled: bool,
    /// `(mod_id, latest version)` when a catalog update is available.
    update: Option<(String, String)>,
    updating: bool,
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
