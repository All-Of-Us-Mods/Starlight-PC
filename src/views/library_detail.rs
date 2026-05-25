use gpui::*;
use log::warn;

use std::path::PathBuf;

use crate::backend::api;
use crate::backend::events::{self, BackendEvent};
use crate::backend::services::bepinex_service::{BepInExProgress, BepInExTargetType};
use crate::backend::services::core_service;
use crate::backend::services::launch_service::{self, LaunchModdedArgs};
use crate::backend::services::profile_service::{self, ProfileEntry, ProfileIconSelection};
use crate::backend::state::game_runtime;
use crate::settings as app_settings;
use crate::theme::{self, ThemeExt};
use crate::ui::icon::AppIcon;
use crate::ui::profile_icon::profile_icon;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::progress::Progress;
use gpui_component::skeleton::Skeleton;
use gpui_component::{Disableable, Sizable as _};
use gpui_component::{Icon, IconName};

#[derive(Clone, Debug)]
pub enum LibraryDetailEvent {
    Close,
}

impl EventEmitter<LibraryDetailEvent> for LibraryDetailView {}

pub struct LibraryDetailView {
    profile_id: String,
    state: LoadState,
    bep_progress: Option<BepInExProgress>,
    confirming_delete: bool,
    launch_error: Option<String>,
    rename_dialog: Option<Entity<InputState>>,
    export_dialog: Option<Entity<InputState>>,
    icon_dialog: Option<IconDialogState>,
    running_count: usize,
    stoppable_count: usize,
    log_filter_input: Entity<InputState>,
    log_view_input: Entity<InputState>,
    log_view_cache: String,
    log_query: String,
    log_filters: LogFilters,
    /// Disk-cached log content + mod-files listing. Refreshed by
    /// [`refresh_disk_state`], NOT in the render loop.
    log_content: String,
    mod_files: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum LogLevel {
    Error,
    Warning,
    Info,
    Message,
    Debug,
    Other,
}

impl LogLevel {
    fn detect(line: &str) -> Self {
        // BepInEx log format: `[<Level>  : <Source>] message`.
        let s = line.trim_start();
        if s.starts_with("[Error") {
            Self::Error
        } else if s.starts_with("[Warning") {
            Self::Warning
        } else if s.starts_with("[Info") {
            Self::Info
        } else if s.starts_with("[Message") {
            Self::Message
        } else if s.starts_with("[Debug") {
            Self::Debug
        } else {
            Self::Other
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
            Self::Info => "Info",
            Self::Message => "Message",
            Self::Debug => "Debug",
            Self::Other => "Other",
        }
    }

    fn chip_id(self) -> &'static str {
        match self {
            Self::Error => "log-level-Error",
            Self::Warning => "log-level-Warning",
            Self::Info => "log-level-Info",
            Self::Message => "log-level-Message",
            Self::Debug => "log-level-Debug",
            Self::Other => "log-level-Other",
        }
    }
}

const LOG_LEVELS: [LogLevel; 6] = [
    LogLevel::Error,
    LogLevel::Warning,
    LogLevel::Message,
    LogLevel::Info,
    LogLevel::Debug,
    LogLevel::Other,
];

struct LogFilters {
    error: bool,
    warning: bool,
    info: bool,
    message: bool,
    debug: bool,
    other: bool,
}

impl Default for LogFilters {
    fn default() -> Self {
        Self {
            error: true,
            warning: true,
            info: true,
            message: true,
            debug: true,
            other: true,
        }
    }
}

impl LogFilters {
    fn is_enabled(&self, level: LogLevel) -> bool {
        match level {
            LogLevel::Error => self.error,
            LogLevel::Warning => self.warning,
            LogLevel::Info => self.info,
            LogLevel::Message => self.message,
            LogLevel::Debug => self.debug,
            LogLevel::Other => self.other,
        }
    }

    fn toggle(&mut self, level: LogLevel) {
        let slot = match level {
            LogLevel::Error => &mut self.error,
            LogLevel::Warning => &mut self.warning,
            LogLevel::Info => &mut self.info,
            LogLevel::Message => &mut self.message,
            LogLevel::Debug => &mut self.debug,
            LogLevel::Other => &mut self.other,
        };
        *slot = !*slot;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IconDialogMode {
    Default,
    Custom,
    Mod,
}

struct IconDialogState {
    mode: IconDialogMode,
    selected_mod_id: Option<String>,
    pending_custom: Option<(Vec<u8>, String)>,
    error: Option<String>,
}

enum LoadState {
    Loading,
    Loaded(ProfileEntry),
    NotFound,
    Failed(String),
}

impl LibraryDetailView {
    pub fn new(profile_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let log_filter_input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter log…"));
        cx.subscribe(&log_filter_input, |this, state, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.log_query = state.read(cx).value().to_string();
                cx.notify();
            }
        })
        .detach();

        // Custom `"log"` language is registered at app startup
        // (see `ui::log_language::register`). Highlights the `[Level: …]`
        // prefix per log level.
        let log_view_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("log")
                .multi_line(true)
                .line_number(false)
                .folding(false)
        });

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
            log_filter_input,
            log_view_input,
            log_view_cache: String::new(),
            log_query: String::new(),
            log_filters: LogFilters::default(),
            log_content: String::new(),
            mod_files: Vec::new(),
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
                    _ => {}
                }
            }
        })
        .detach();

        view
    }

    fn spawn_load(&self, cx: &mut Context<Self>) {
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
            });
        })
        .detach();
    }

    /// Reload the on-disk artifacts shown on this page (log file + mods
    /// listing). Cheap to call repeatedly — runs on the background executor
    /// and notifies once both reads finish, so we never touch the disk inside
    /// `render()`.
    fn refresh_disk_state(&self, cx: &mut Context<Self>) {
        let LoadState::Loaded(profile) = &self.state else {
            return;
        };
        let path = profile.path.clone();
        cx.spawn(async move |this, cx| {
            let (log, mods) = cx
                .background_executor()
                .spawn(async move {
                    let log = profile_service::get_profile_log(&path, "LogOutput.log");
                    let mods = profile_service::get_mod_files(&path);
                    (log, mods)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.log_content = log;
                this.mod_files = mods;
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle_log_level(&mut self, level: LogLevel, cx: &mut Context<Self>) {
        self.log_filters.toggle(level);
        cx.notify();
    }

    fn render_log_panel(
        &mut self,
        theme: &crate::theme::Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Cap retained lines so the filter pass stays cheap on huge logs.
        const MAX_LINES: usize = 2000;

        let query = self.log_query.trim().to_lowercase();
        let total_count = self.log_content.lines().count();
        let skip = total_count.saturating_sub(MAX_LINES);

        // Filter once, borrowing the log content; no per-line String alloc.
        let kept: Vec<&str> = self
            .log_content
            .lines()
            .skip(skip)
            .filter(|line| {
                let level = LogLevel::detect(line);
                self.log_filters.is_enabled(level)
                    && (query.is_empty() || line.to_lowercase().contains(&query))
            })
            .collect();
        let kept_count = kept.len();
        let joined = kept.join("\n");

        // Only push text into the Input when the content actually changes —
        // otherwise we'd clobber the user's selection every render.
        if joined != self.log_view_cache {
            let new_text = joined.clone();
            self.log_view_input.update(cx, |state, cx| {
                state.set_value(new_text, window, cx);
            });
            self.log_view_cache = joined;
        }

        let filter_chips: Vec<AnyElement> = LOG_LEVELS
            .iter()
            .copied()
            .map(|level| {
                let active = self.log_filters.is_enabled(level);
                let mut btn = Button::new(level.chip_id()).xsmall().label(level.label());
                if active {
                    btn = btn.primary();
                } else {
                    btn = btn.ghost();
                }
                btn.on_click(cx.listener(move |this, _, _window, cx| {
                    this.toggle_log_level(level, cx);
                }))
                .into_any_element()
            })
            .collect();

        let lines_for_copy = self.log_view_cache.clone();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Latest log"))
                    .child(div().text_xs().text_color(theme.text_muted).child(format!(
                        "{kept_count} / {total_count} line{}",
                        if total_count == 1 { "" } else { "s" }
                    ))),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(220.0)).child(Input::new(&self.log_filter_input)))
                    .children(filter_chips)
                    .child(
                        Button::new("copy-log")
                            .xsmall()
                            .ghost()
                            .label("Copy")
                            .on_click(move |_, _window, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    lines_for_copy.clone(),
                                ));
                            }),
                    ),
            )
            .child(
                div().h(px(320.0)).child(
                    Input::new(&self.log_view_input)
                        .font_family("ui-monospace, monospace")
                        .size_full(),
                ),
            )
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

    fn launch(&mut self, cx: &mut Context<Self>) {
        let LoadState::Loaded(profile) = &self.state else {
            return;
        };
        let profile = profile.clone();
        self.launch_error = None;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { run_launch(profile) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    warn!("launch failed: {e}");
                    this.launch_error = Some(e);
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

    fn open_icon_dialog(&mut self, cx: &mut Context<Self>) {
        let LoadState::Loaded(profile) = &self.state else {
            return;
        };
        let mode = match profile.icon_mode.as_deref() {
            Some("custom") => IconDialogMode::Custom,
            Some("mod") => IconDialogMode::Mod,
            _ => IconDialogMode::Default,
        };
        let selected_mod_id = profile
            .icon_mod_id
            .clone()
            .or_else(|| profile.mods.first().map(|m| m.mod_id.clone()));
        self.icon_dialog = Some(IconDialogState {
            mode,
            selected_mod_id,
            pending_custom: None,
            error: None,
        });
        cx.notify();
    }

    fn set_icon_mode(&mut self, mode: IconDialogMode, cx: &mut Context<Self>) {
        if let Some(state) = self.icon_dialog.as_mut() {
            state.mode = mode;
            state.error = None;
            if mode == IconDialogMode::Mod && state.selected_mod_id.is_none() {
                if let LoadState::Loaded(profile) = &self.state {
                    state.selected_mod_id = profile.mods.first().map(|m| m.mod_id.clone());
                }
            }
            cx.notify();
        }
    }

    fn pick_custom_icon(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose icon image".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| format!(".{}", s.to_lowercase()))
                .unwrap_or_default();
            let read = cx
                .background_executor()
                .spawn(async move { std::fs::read(&path) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Some(state) = this.icon_dialog.as_mut() {
                    match read {
                        Ok(bytes) if !bytes.is_empty() => {
                            state.pending_custom = Some((bytes, extension));
                            state.error = None;
                        }
                        Ok(_) => state.error = Some("Selected image is empty".into()),
                        Err(e) => state.error = Some(format!("Failed to read image: {e}")),
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn save_icon(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.icon_dialog.as_ref() else {
            return;
        };
        let selection = match state.mode {
            IconDialogMode::Default => ProfileIconSelection::Default,
            IconDialogMode::Custom => {
                let LoadState::Loaded(profile) = &self.state else {
                    return;
                };
                let has_existing = profile.icon_mode.as_deref() == Some("custom")
                    && profile.custom_icon_extension.is_some();
                match state.pending_custom.clone() {
                    Some((bytes, extension)) => ProfileIconSelection::Custom { bytes, extension },
                    None if has_existing => {
                        self.icon_dialog = None;
                        cx.notify();
                        return;
                    }
                    None => {
                        if let Some(s) = self.icon_dialog.as_mut() {
                            s.error = Some("Choose an image for the custom icon".into());
                        }
                        cx.notify();
                        return;
                    }
                }
            }
            IconDialogMode::Mod => {
                let Some(mod_id) = state.selected_mod_id.clone() else {
                    if let Some(s) = self.icon_dialog.as_mut() {
                        s.error = Some("Select an installed mod icon".into());
                    }
                    cx.notify();
                    return;
                };
                ProfileIconSelection::Mod { mod_id }
            }
        };

        let id = self.profile_id.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::update_profile_icon(&id, selection) })
                .await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(()) => {
                    this.icon_dialog = None;
                    this.spawn_load(cx);
                }
                Err(e) => {
                    if let Some(s) = this.icon_dialog.as_mut() {
                        s.error = Some(format!("Failed to update icon: {e}"));
                    }
                    cx.notify();
                }
            });
        })
        .detach();
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

fn run_launch(profile: ProfileEntry) -> Result<(), String> {
    let settings = core_service::get_settings().map_err(|e| e.to_string())?;
    let game_path = settings.among_us_path.trim();
    if game_path.is_empty() {
        return Err("Among Us path is not set. Configure it in Settings.".into());
    }

    let game_exe = PathBuf::from(game_path).join(GAME_EXE_NAME);
    if !game_exe.exists() {
        return Err(format!(
            "{} not found at {}",
            GAME_EXE_NAME,
            game_exe.display()
        ));
    }

    let profile_path = PathBuf::from(&profile.path);
    let bepinex_dll = profile_path
        .join("BepInEx")
        .join("core")
        .join("BepInEx.Unity.IL2CPP.dll");
    if !bepinex_dll.exists() {
        return Err("BepInEx DLL not found. Install BepInEx for this profile first.".into());
    }
    let dotnet_dir = profile_path.join("dotnet");
    let coreclr_path = dotnet_dir.join(CORECLR_FILE);
    if !coreclr_path.exists() {
        return Err(format!(
            "dotnet runtime not found at {}",
            coreclr_path.display()
        ));
    }

    let platform_label = match settings.game_platform {
        core_service::GamePlatform::Steam => "steam",
        core_service::GamePlatform::Epic => "epic",
        core_service::GamePlatform::Xbox => "xbox",
    };

    #[cfg(target_os = "linux")]
    let runner = build_linux_runner(&settings)?;

    let args = LaunchModdedArgs {
        game_exe: game_exe.to_string_lossy().to_string(),
        profile_id: profile.id.clone(),
        #[cfg(any(windows, target_os = "linux"))]
        profile_path: profile.path.clone(),
        bepinex_dll: bepinex_dll.to_string_lossy().to_string(),
        dotnet_dir: dotnet_dir.to_string_lossy().to_string(),
        coreclr_path: coreclr_path.to_string_lossy().to_string(),
        platform: platform_label.to_string(),
        #[cfg(target_os = "linux")]
        runner,
    };

    launch_service::launch_modded(args).map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
const GAME_EXE_NAME: &str = "Among Us.exe";
#[cfg(not(target_os = "windows"))]
const GAME_EXE_NAME: &str = "Among Us.exe";

#[cfg(target_os = "windows")]
const CORECLR_FILE: &str = "coreclr.dll";
#[cfg(target_os = "linux")]
const CORECLR_FILE: &str = "coreclr.dll";
#[cfg(target_os = "macos")]
const CORECLR_FILE: &str = "libcoreclr.dylib";

#[cfg(target_os = "linux")]
fn build_linux_runner(
    settings: &core_service::AppSettings,
) -> Result<launch_service::LinuxRunner, String> {
    let binary = settings.linux_runner_binary.trim();
    if binary.is_empty() {
        return Err("Linux runner binary is required in Settings.".into());
    }
    Ok(match settings.linux_runner_kind {
        core_service::LinuxRunnerKind::Wine => launch_service::LinuxRunner::Wine {
            binary: binary.to_string(),
            prefix: settings.linux_wine_prefix.clone(),
        },
        core_service::LinuxRunnerKind::Proton => launch_service::LinuxRunner::Proton {
            binary: binary.to_string(),
            compat_data_path: settings.linux_proton_compat_data_path.clone(),
            steam_client_path: settings.linux_proton_steam_client_path.clone(),
            use_steam_run: settings.linux_proton_use_steam_run,
        },
    })
}

impl Render for LibraryDetailView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let back = Button::new("back")
            .ghost()
            .icon(Icon::new(IconName::ArrowLeft))
            .label("Back")
            .on_click(cx.listener(|_, _, _window, cx| {
                cx.emit(LibraryDetailEvent::Close);
            }));

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
                    let mut row = div().flex().gap_2().items_center();
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
                let has_log = !self.log_content.is_empty();
                let mod_files = self.mod_files.clone();

                let mods_section = (!profile.mods.is_empty()).then(|| {
                    let entries: Vec<AnyElement> = profile
                        .mods
                        .iter()
                        .map(|m| {
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .px_3()
                                .py_2()
                                .border_b_1()
                                .border_color(theme.border)
                                .child(div().child(m.mod_id.clone()))
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
                        .pt_4()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .pb_2()
                                .child(format!("Mods ({})", profile.mods.len())),
                        )
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

                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(profile_icon(profile, 64.0, &theme))
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child(profile.name.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
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
                                Button::new("export-profile-action")
                                    .label("Export ZIP")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_export_dialog(window, cx);
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(profile.path.clone()),
                    )
                    .children((!bep_installed).then(|| {
                        div()
                            .text_xs()
                            .text_color(rgb(0xf59e0b))
                            .child("BepInEx not installed")
                    }))
                    .children(progress_row)
                    .children(install_btn)
                    .children(launch_row)
                    .children(launch_err)
                    .child(
                        div()
                            .grid()
                            .grid_cols(3)
                            .gap_3()
                            .child(stat_card("Created", profile.created_at.to_string(), &theme))
                            .child(stat_card(
                                "Last launched",
                                profile
                                    .last_launched_at
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "Never".to_string()),
                                &theme,
                            ))
                            .child(stat_card(
                                "Play time",
                                format!("{} min", profile.total_play_time.unwrap_or(0) / 60_000),
                                &theme,
                            )),
                    )
                    .children(mods_section)
                    .children((!mod_files.is_empty()).then(|| {
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Plugin files"),
                            )
                            .child(
                                div()
                                    .rounded_lg()
                                    .bg(theme.sidebar_background)
                                    .border_1()
                                    .border_color(theme.border)
                                    .children(mod_files.into_iter().map(|file| {
                                        div()
                                            .px_3()
                                            .py_2()
                                            .border_b_1()
                                            .border_color(theme.border)
                                            .child(file)
                                    })),
                            )
                    }))
                    .children(
                        has_log.then(|| self.render_log_panel(&theme, window, cx)),
                    )
                    .child(delete_row)
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

fn render_icon_dialog(
    state: &IconDialogState,
    profile: &ProfileEntry,
    theme: crate::theme::Theme,
    cx: &mut Context<LibraryDetailView>,
) -> AnyElement {
    let mode = state.mode;
    let mode_button = |id: &'static str, label: &'static str, target: IconDialogMode| {
        let mut btn = Button::new(id).label(label).on_click(
            cx.listener(move |this, _: &ClickEvent, _, cx| this.set_icon_mode(target, cx)),
        );
        if mode == target {
            btn = btn.primary();
        }
        btn
    };

    let mode_row = div()
        .flex()
        .gap_2()
        .child(mode_button(
            "icon-mode-default",
            "Default",
            IconDialogMode::Default,
        ))
        .child(mode_button(
            "icon-mode-custom",
            "Custom Image",
            IconDialogMode::Custom,
        ))
        .child(mode_button(
            "icon-mode-mod",
            "Installed Mod",
            IconDialogMode::Mod,
        ));

    let body: AnyElement = match mode {
        IconDialogMode::Default => div()
            .text_sm()
            .text_color(theme.text_muted)
            .child("Use the default profile icon.")
            .into_any_element(),
        IconDialogMode::Custom => {
            let has_pending = state.pending_custom.is_some();
            let has_existing = profile.icon_mode.as_deref() == Some("custom")
                && profile.custom_icon_extension.is_some();
            let status: AnyElement = if has_pending {
                div()
                    .text_sm()
                    .child("New image ready to save.")
                    .into_any_element()
            } else if has_existing {
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child("Using existing custom image. Choose a new one to replace it.")
                    .into_any_element()
            } else {
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child("PNG, JPG, WEBP, GIF, BMP, or AVIF.")
                    .into_any_element()
            };
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    Button::new("icon-pick-file")
                        .icon(Icon::new(IconName::FolderOpen))
                        .label(if has_pending || has_existing {
                            "Change Image"
                        } else {
                            "Choose Image"
                        })
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.pick_custom_icon(window, cx);
                        })),
                )
                .child(status)
                .into_any_element()
        }
        IconDialogMode::Mod => {
            let mods: Vec<String> = profile
                .mods
                .iter()
                .map(|m| m.mod_id.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            if mods.is_empty() {
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child("No mods installed. Add a mod to use its icon.")
                    .into_any_element()
            } else {
                let selected = state.selected_mod_id.clone();
                let theme_for_items = theme.clone();
                let items: Vec<AnyElement> = mods
                    .into_iter()
                    .map(|mod_id| {
                        let is_selected = selected.as_deref() == Some(mod_id.as_str());
                        let click_id = mod_id.clone();
                        div()
                            .id(SharedString::from(format!("icon-mod-{mod_id}")))
                            .flex()
                            .items_center()
                            .gap_2()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(if is_selected {
                                theme_for_items.primary
                            } else {
                                theme_for_items.border
                            })
                            .cursor_pointer()
                            .hover(|s| s.bg(theme_for_items.hover))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                if let Some(s) = this.icon_dialog.as_mut() {
                                    s.selected_mod_id = Some(click_id.clone());
                                    s.error = None;
                                    cx.notify();
                                }
                            }))
                            .child(
                                img(api::mod_thumbnail_url(&mod_id))
                                    .w(px(36.0))
                                    .h(px(36.0))
                                    .rounded_md()
                                    .object_fit(ObjectFit::Cover),
                            )
                            .child(div().text_sm().truncate().child(mod_id))
                            .into_any_element()
                    })
                    .collect();
                div()
                    .id("icon-mod-list")
                    .max_h(px(240.0))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(items)
                    .into_any_element()
            }
        }
    };

    let error_row = state
        .error
        .clone()
        .map(|msg| div().text_sm().text_color(rgb(0xef4444)).child(msg));

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
                .w(px(480.0))
                .p_5()
                .rounded_lg()
                .bg(theme.background)
                .border_1()
                .border_color(theme.border)
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Edit Profile Icon"),
                )
                .child(mode_row)
                .child(body)
                .children(error_row)
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .justify_end()
                        .child(Button::new("icon-dialog-cancel").label("Cancel").on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.icon_dialog = None;
                                cx.notify();
                            }),
                        ))
                        .child(
                            Button::new("icon-dialog-save")
                                .primary()
                                .label("Save")
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.save_icon(cx);
                                })),
                        ),
                ),
        )
        .into_any_element()
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
