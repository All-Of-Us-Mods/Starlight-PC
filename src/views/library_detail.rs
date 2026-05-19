use gpui::*;
use log::warn;

use std::path::PathBuf;

use crate::backend::events::{self, BackendEvent};
use crate::backend::services::bepinex_service::{BepInExProgress, BepInExTargetType};
use crate::backend::services::core_service;
use crate::backend::services::launch_service::{self, LaunchModdedArgs};
use crate::backend::services::profile_service::{self, ProfileEntry};
use crate::theme::{self, ThemeExt};
use crate::ui::icon::AppIcon;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::progress::Progress;
use gpui_component::skeleton::Skeleton;
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
}

enum LoadState {
    Loading,
    Loaded(ProfileEntry),
    NotFound,
    Failed(String),
}

impl LibraryDetailView {
    pub fn new(profile_id: String, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let view = Self {
            profile_id: profile_id.clone(),
            state: LoadState::Loading,
            bep_progress: None,
            confirming_delete: false,
            launch_error: None,
            rename_dialog: None,
            export_dialog: None,
        };

        view.spawn_load(cx);

        // Subscribe to BepInEx progress for *this* profile.
        let id_for_events = profile_id.clone();
        let mut rx = events::subscribe();
        cx.spawn(async move |this, cx| {
            while let Ok(event) = rx.recv().await {
                if let BackendEvent::BepInExProgress(p) = event
                    && matches!(p.target_type, BepInExTargetType::Profile)
                    && p.target_id == id_for_events
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
        let state =
            cx.new(|cx| InputState::new(window, cx).placeholder("Destination .zip path"));
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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

                let launch_btn = bep_installed.then(|| {
                    Button::new("launch")
                        .success()
                        .icon(Icon::new(IconName::Play))
                        .label("Launch")
                        .on_click(cx.listener(|this, _, _window, cx| this.launch(cx)))
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
                let profile_log = profile_service::get_profile_log(&profile.path, "LogOutput.log");
                let mod_files = profile_service::get_mod_files(&profile.path);

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

                let delete_row = if self.confirming_delete {
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
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.delete_profile(cx)
                                })),
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
                        div().flex().items_center().justify_between().child(
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
                            .child(Button::new("rename-profile-action").label("Rename").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.open_rename_dialog(window, cx);
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
                    .children(launch_btn)
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
                    .children((!profile_log.is_empty()).then(|| {
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Latest log"))
                            .child(
                                div()
                                    .max_h(px(220.0))
                                    .overflow_hidden()
                                    .rounded_lg()
                                    .bg(theme.sidebar_background)
                                    .border_1()
                                    .border_color(theme.border)
                                    .p_3()
                                    .text_xs()
                                    .child(
                                        profile_log
                                            .chars()
                                            .rev()
                                            .take(4000)
                                            .collect::<String>()
                                            .chars()
                                            .rev()
                                            .collect::<String>(),
                                    ),
                            )
                    }))
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
    }
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
