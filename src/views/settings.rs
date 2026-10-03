mod installations;

use std::rc::Rc;

use gpui_kit::component::{
    AxisExt as _, Icon, IconName, Sizable as _, WindowExt,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
    notification::Notification,
    progress::Progress,
    scroll::ScrollableElement as _,
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use log::warn;

use crate::backend::binary::BinaryArch;
use crate::backend::events::{self, BackendEvent};
use crate::backend::services::core_service::ReleaseChannel;
use crate::backend::services::{
    bepinex_service::{self, BepInExTargetType},
    core_service::{self, GamePlatform, ScrollbarVisibility},
};
#[cfg(unix)]
use crate::backend::services::{core_service::LinuxRunnerKind, finder_service};
use crate::settings as app_settings;
use crate::ui::icon::AppIcon;
use crate::updater::{self, UpdateState};
use gpui_kit::component::ActiveTheme;
use rust_i18n::t;

type PathSetter = Rc<dyn Fn(SharedString, &mut App)>;

/// (locale code, native display name) pairs for the Settings → Appearance
/// language dropdown. Codes come from the locale files under `locales/`; a
/// code without an entry in `NAMES` is shown as-is.
fn language_options() -> Vec<(SharedString, SharedString)> {
    const NAMES: &[(&str, &str)] = &[
        ("ar", "العربية"),
        ("en", "English"),
        ("nl", "Nederlands"),
        ("de", "Deutsch"),
        ("fr", "Français"),
        ("es", "Español"),
        ("pt-BR", "Português (Brasil)"),
        ("ru", "Русский"),
        ("ja", "日本語"),
        ("zh-CN", "简体中文"),
        ("zh-TW", "繁體中文"),
    ];
    rust_i18n::available_locales!()
        .into_iter()
        .map(|code| {
            let name = NAMES
                .iter()
                .find(|(known, _)| *known == code.as_ref())
                .map(|(_, name)| *name)
                .unwrap_or(code.as_ref())
                .to_string();
            (code.into_owned().into(), name.into())
        })
        .collect()
}

pub struct SettingsView;

impl SettingsView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.observe_global::<app_settings::SettingsGlobal>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<updater::UpdateGlobal>(|_, cx| cx.notify())
            .detach();

        // Refresh on cache state changes (download / clear).
        events::listen(cx, |_, event, cx| {
            if let BackendEvent::BepInExProgress(p) = event
                && matches!(p.target_type, BepInExTargetType::Cache)
            {
                cx.notify();
            }
        });

        Self
    }
}

/// A setting item whose field is rendered under the label instead of beside
/// it. The horizontal layout caps the label at 60% of the row and clips
/// whatever doesn't fit in the rest, so anything wider than a switch or a
/// short dropdown — text inputs, path pickers, button pairs — has to stack.
fn stacked_item(title: impl Into<SharedString>, field: SettingField<SharedString>) -> SettingItem {
    SettingItem::new(title, field).layout(Axis::Vertical)
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.2} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.1} KiB", b / KIB)
    } else {
        format!("{bytes} B")
    }
}

/// Build the download/clear row + status description for one BepInEx cache
/// architecture. The cache is sized once here and reused for both the "Clear"
/// button's visibility and the description, instead of stat-ing the file twice.
fn cache_item(arch: BinaryArch, label: gpui_kit::SharedString) -> SettingItem {
    let (present, status): (bool, SharedString) = match core_service::get_bepinex_cache_path(arch) {
        Ok(path) => match bepinex_service::cache_size(&path) {
            Some(size) => (
                true,
                t!("settings.cache.cached", size = format_bytes(size)).into(),
            ),
            None => (false, t!("settings.cache.not_cached").into()),
        },
        Err(_) => (false, t!("settings.cache.path_unavailable").into()),
    };
    stacked_item(
        label,
        SettingField::render(move |_, _, _| {
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(
                    Button::new(SharedString::from(format!("cache-{}", arch.as_str())))
                        .icon(Icon::new(AppIcon::Download))
                        .label(t!("settings.cache.download"))
                        .on_click(move |_, window, cx| download_bepinex_cache(arch, window, cx)),
                )
                .when(present, |row| {
                    row.child(
                        Button::new(SharedString::from(format!("clear-{}", arch.as_str())))
                            .danger()
                            .icon(Icon::new(IconName::Delete))
                            .label(t!("common.clear"))
                            .on_click(move |_, window, cx| clear_bepinex_cache(arch, window, cx)),
                    )
                })
        }),
    )
    .description(status)
}

fn patch_platform(value: SharedString, cx: &mut App) {
    if let Some(platform) = GamePlatform::from_id(&value) {
        app_settings::update(cx, |s| s.game.game_platform = platform);
    }
}

fn patch_theme_name(value: SharedString, cx: &mut App) {
    app_settings::update(cx, |s| s.theme_name = value.to_string());
    crate::theme::apply(cx, &value);
}

fn patch_language(value: SharedString, cx: &mut App) {
    app_settings::update(cx, |s| s.language = value.to_string());
    rust_i18n::set_locale(&value);
    // The sidebar and title bar live outside this view's tree and don't
    // observe settings, so force everything to re-render in the new locale.
    cx.refresh_windows();
}

fn patch_show_stars_background(value: bool, cx: &mut App) {
    app_settings::update(cx, |s| s.show_stars_background = value);
    // The stars layer lives in the workspace, which doesn't observe settings.
    cx.refresh_windows();
}

fn patch_scrollbar_visibility(value: SharedString, cx: &mut App) {
    let visibility = match value.as_ref() {
        "hover" => ScrollbarVisibility::Hover,
        "always" => ScrollbarVisibility::Always,
        _ => ScrollbarVisibility::Scrolling,
    };
    app_settings::update(cx, |s| s.scrollbar_visibility = visibility);
    crate::theme::apply_scrollbar_visibility(cx);
    cx.refresh_windows();
}

fn patch_release_channel(value: SharedString, cx: &mut App) {
    let channel = match value.as_ref() {
        "nightly" => ReleaseChannel::Nightly,
        _ => ReleaseChannel::Stable,
    };
    app_settings::update(cx, |s| s.release_channel = channel);
}

/// The updater's current phase, as the description under "Check for updates".
fn update_status(state: &UpdateState) -> String {
    match state {
        UpdateState::Idle => t!("settings.check_for_updates_desc").to_string(),
        UpdateState::Checking => t!("update.checking").to_string(),
        UpdateState::UpToDate => t!("update.up_to_date").to_string(),
        UpdateState::Available(info) => t!("update.available", version = info.version).to_string(),
        UpdateState::Downloading { info, percent } => {
            let status = t!("update.downloading", version = info.version);
            match percent {
                Some(percent) => format!("{status} — {percent}%"),
                None => status.to_string(),
            }
        }
        UpdateState::Failed {
            error,
            update: Some(_),
        } => t!("update.failed", error = error).to_string(),
        UpdateState::Failed {
            error,
            update: None,
        } => t!("update.check_failed", error = error).to_string(),
    }
}

/// The action for the updater's current phase: check (or retry the check),
/// install (or retry the install), or the download in progress.
fn render_update_controls(cx: &App) -> AnyElement {
    match updater::state(cx) {
        UpdateState::Available(info)
        | UpdateState::Failed {
            update: Some(info), ..
        } => updater::install_button(info.clone(), cx).into_any_element(),
        UpdateState::Downloading { info, percent } => div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                Progress::new("update-progress")
                    .w_32()
                    .value(percent.unwrap_or(0) as f32)
                    .loading(percent.is_none()),
            )
            .child(updater::install_button(info.clone(), cx))
            .into_any_element(),
        state => Button::new("check-for-updates")
            .icon(Icon::new(AppIcon::Download))
            .label(t!("settings.check_now"))
            .loading(matches!(state, UpdateState::Checking))
            .on_click(|_, window, cx| updater::check(window, cx))
            .into_any_element(),
    }
}

#[cfg(unix)]
fn patch_linux_runner_kind(value: SharedString, cx: &mut App) {
    let kind = match value.as_ref() {
        "wine" => LinuxRunnerKind::Wine,
        "steam" => LinuxRunnerKind::Steam,
        _ => LinuxRunnerKind::Proton,
    };
    app_settings::update(cx, |s| s.game.linux_runner_kind = kind);
}

// ---------- path input field (Input + Browse button, two-way bound) ----------

struct PathFieldState {
    input: Entity<InputState>,
    /// The setting's value as last mirrored into `input`. Only a change to
    /// the setting itself (Browse, Auto-detect) overwrites the input, so a
    /// re-render mid-edit leaves the user's unsaved text alone.
    synced: SharedString,
    _sub: Subscription,
}

/// File-path setting field. The input mirrors the global (so an external
/// write like Auto-detect updates the visible text), edits are saved through
/// `set` on Enter or when the input loses focus — not on every keystroke —
/// and the Browse button opens the platform file picker.
fn path_field(
    key: &'static str,
    directories_only: bool,
    get: fn(&App) -> SharedString,
    set: fn(SharedString, &mut App),
) -> SettingField<SharedString> {
    SettingField::render(move |options, window, cx| {
        let value = get(cx);

        let state_key = SharedString::from(format!("path-field-{key}"));
        let value_for_init = value.clone();
        let state = window.use_keyed_state(state_key, cx, move |window, cx| {
            let input =
                cx.new(|cx| InputState::new(window, cx).default_value(value_for_init.clone()));
            let _sub = cx.subscribe(&input, move |_, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    let edited = input.read(cx).value();
                    if edited != get(cx) {
                        set(edited, cx);
                    }
                }
            });
            PathFieldState {
                input,
                synced: value_for_init,
                _sub,
            }
        });

        let input_entity = state.read(cx).input.clone();
        if state.read(cx).synced != value {
            state.update(cx, |state, _| state.synced = value.clone());
            if input_entity.read(cx).value() != value {
                let val = value.clone();
                input_entity.update(cx, |s, cx| s.set_value(val, window, cx));
            }
        }

        let prompt: SharedString = if directories_only {
            t!("settings.path.select_folder").into()
        } else {
            t!("settings.path.select_file").into()
        };
        let button_id = SharedString::from(format!("path-browse-{key}"));
        let setter: PathSetter = Rc::new(set);

        let input_el = Input::new(&input_entity)
            .with_size(options.size())
            .map(|this| {
                if options.layout().is_horizontal() {
                    this.w_64()
                } else {
                    this.w_full()
                }
            });

        div().flex().gap_2().child(input_el).child(
            Button::new(button_id)
                .icon(Icon::new(IconName::FolderOpen))
                .label(t!("settings.path.browse"))
                .with_size(options.size())
                .on_click(move |_, window, cx| {
                    let receiver = cx.prompt_for_paths(PathPromptOptions {
                        files: !directories_only,
                        directories: directories_only,
                        multiple: false,
                        prompt: Some(prompt.clone()),
                    });
                    let setter = setter.clone();
                    window
                        .spawn(cx, async move |cx| {
                            let Ok(Ok(Some(paths))) = receiver.await else {
                                return;
                            };
                            let Some(path) = paths.into_iter().next() else {
                                return;
                            };
                            let s: SharedString = path.to_string_lossy().into_owned().into();
                            let _ = cx.update(|_, cx| setter(s, cx));
                        })
                        .detach();
                }),
        )
    })
}

// ---------- action handlers (Detect / Cache / Clear) ----------

#[cfg(unix)]
fn detect_linux_runtime(window: &mut Window, cx: &mut App) {
    let among_us_path = app_settings::get(cx).game.among_us_path.clone();
    let path_arg = (!among_us_path.trim().is_empty()).then_some(among_us_path);
    let detection = cx
        .background_executor()
        .spawn(async move { finder_service::detect_linux_runner(path_arg) });
    window
        .spawn(cx, async move |cx| {
            let detection = detection.await;
            let _ = cx.update(|window, cx| match detection {
                Ok(detection) => {
                    app_settings::update(cx, |s| {
                        s.game.linux_runner_kind = detection.runner_kind;
                        s.game.linux_runner_binary = detection.runner_binary.unwrap_or_default();
                        s.game.linux_wine_prefix = detection.wine_prefix.unwrap_or_default();
                        s.game.linux_proton_compat_data_path =
                            detection.proton_compat_data_path.unwrap_or_default();
                        s.game.linux_proton_steam_client_path =
                            detection.proton_steam_client_path.unwrap_or_default();
                        s.game.linux_proton_use_steam_run = detection.proton_use_steam_run;
                    });
                    window.push_notification(
                        Notification::success(t!("settings.linux.detected").to_string()),
                        cx,
                    );
                }
                Err(e) => {
                    warn!("detect_linux_runner failed: {e}");
                    window.push_notification(
                        Notification::error(t!("settings.detection_failed", error = e).to_string()),
                        cx,
                    );
                }
            });
        })
        .detach();
}

fn detect_among_us(window: &mut Window, cx: &mut App) {
    let detection = app_settings::detect_among_us(cx);
    window
        .spawn(cx, async move |cx| {
            let detection = detection.await;
            let _ = cx.update(|window, cx| match detection {
                Ok(Some((path, store))) => {
                    let msg = match store {
                        Some(p) => t!(
                            "settings.detected_store",
                            store = p.display_name(),
                            path = path
                        )
                        .to_string(),
                        None => t!("settings.detected", path = path).to_string(),
                    };
                    window.push_notification(Notification::success(msg), cx);
                }
                Ok(None) => {
                    window.push_notification(
                        Notification::warning(t!("settings.not_detected").to_string()),
                        cx,
                    );
                }
                Err(e) => {
                    warn!("detect_among_us failed: {e}");
                    window.push_notification(
                        Notification::error(t!("settings.detection_failed", error = e).to_string()),
                        cx,
                    );
                }
            });
        })
        .detach();
}

fn download_bepinex_cache(arch: BinaryArch, window: &mut Window, cx: &mut App) {
    let url = app_settings::get(cx).bepinex_url(arch).to_string();
    let cache_path = match core_service::get_bepinex_cache_path(arch) {
        Ok(p) => p,
        Err(e) => {
            window.push_notification(
                Notification::error(t!("settings.cache.path_error", error = e).to_string()),
                cx,
            );
            return;
        }
    };
    let window_handle = window.window_handle();
    cx.spawn(async move |cx| {
        let result = cx
            .background_executor()
            .spawn(async move {
                bepinex_service::download_bepinex_to_cache(
                    url,
                    cache_path,
                    arch.as_str().to_string(),
                )
            })
            .await;
        let _ = window_handle.update(cx, |_, window, cx| match result {
            Ok(()) => window.push_notification(
                Notification::success(
                    t!("settings.cache.downloaded", arch = arch.as_str()).to_string(),
                ),
                cx,
            ),
            Err(e) => {
                warn!("BepInEx cache download ({}) failed: {e}", arch.as_str());
                window.push_notification(
                    Notification::error(
                        t!(
                            "settings.cache.download_failed",
                            arch = arch.as_str(),
                            error = e
                        )
                        .to_string(),
                    ),
                    cx,
                );
            }
        });
    })
    .detach();
}

fn clear_bepinex_cache(arch: BinaryArch, window: &mut Window, cx: &mut App) {
    let path = match core_service::get_bepinex_cache_path(arch) {
        Ok(path) => path,
        Err(e) => {
            window.push_notification(
                Notification::error(t!("settings.cache.path_error", error = e).to_string()),
                cx,
            );
            return;
        }
    };
    let cleared = cx
        .background_executor()
        .spawn(async move { bepinex_service::clear_cache(path, arch.as_str().to_string()) });
    window
        .spawn(cx, async move |cx| {
            let cleared = cleared.await;
            let _ = cx.update(|window, cx| match cleared {
                Ok(()) => window.push_notification(
                    Notification::success(
                        t!("settings.cache.cleared", arch = arch.as_str()).to_string(),
                    ),
                    cx,
                ),
                Err(e) => {
                    warn!("clear_bepinex_cache failed: {e}");
                    window.push_notification(
                        Notification::error(
                            t!("settings.cache.clear_failed", error = e).to_string(),
                        ),
                        cx,
                    );
                }
            });
        })
        .detach();
}

/// Open the app's data directory (settings, profiles, logs) in the platform
/// file manager — the folder support asks users to look in.
fn open_data_folder(cx: &App) {
    let Ok(dir) = crate::backend::directories::app_data_dir() else {
        return;
    };
    open_folder(&dir, cx);
}

/// Open `dir` in the platform file manager, creating it first if needed.
fn open_folder(dir: &std::path::Path, cx: &App) {
    let _ = std::fs::create_dir_all(dir);
    cx.open_with_system(dir);
}

// ---------- view ----------

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let mut game_groups = vec![
            SettingGroup::new()
                .title(t!("settings.group.installation"))
                .items(vec![
                    stacked_item(
                        t!("settings.among_us_path"),
                        path_field(
                            "among-us",
                            true,
                            |cx| app_settings::get(cx).game.among_us_path.clone().into(),
                            |value, cx| {
                                app_settings::update(cx, |s| {
                                    s.game.among_us_path = value.to_string()
                                })
                            },
                        ),
                    )
                    .description(t!("settings.among_us_path_desc").to_string()),
                    stacked_item(
                        t!("settings.auto_detect"),
                        SettingField::render(|_, _, _| {
                            Button::new("detect-among-us")
                                .icon(Icon::new(AppIcon::Compass))
                                .label(t!("settings.auto_detect_among_us"))
                                .on_click(|_, window, cx| detect_among_us(window, cx))
                        }),
                    )
                    .description(t!("settings.auto_detect_desc").to_string()),
                ]),
            SettingGroup::new()
                .title(t!("settings.group.platform"))
                .items(vec![
                    SettingItem::new(
                        t!("settings.game_platform"),
                        SettingField::dropdown(
                            GamePlatform::ALL
                                .into_iter()
                                .map(|p| (p.id().into(), p.display_name().into()))
                                .collect(),
                            |cx| app_settings::get(cx).game.game_platform.id().into(),
                            patch_platform,
                        ),
                    )
                    .description(t!("settings.game_platform_desc").to_string()),
                    SettingItem::new(
                        t!("settings.multiple_installations"),
                        SettingField::switch(
                            |cx| app_settings::get(cx).show_installation_controls,
                            |value, cx| {
                                app_settings::update(cx, |s| s.show_installation_controls = value)
                            },
                        ),
                    )
                    .description(t!("settings.multiple_installations_desc").to_string()),
                ]),
        ];
        if app_settings::get(cx).show_installation_controls {
            game_groups.push(installations::group());
        }
        let game_page = SettingPage::new(t!("settings.page.game"))
            .default_open(true)
            .groups(game_groups);

        let launch_items = vec![
            SettingItem::new(
                t!("settings.close_on_launch"),
                SettingField::switch(
                    |cx| app_settings::get(cx).close_on_launch,
                    |value, cx| app_settings::update(cx, |s| s.close_on_launch = value),
                ),
            )
            .description(t!("settings.close_on_launch_desc").to_string()),
            SettingItem::new(
                t!("settings.multi_instance"),
                SettingField::switch(
                    |cx| app_settings::get(cx).allow_multi_instance_launch,
                    |value, cx| app_settings::update(cx, |s| s.allow_multi_instance_launch = value),
                ),
            )
            .description(t!("settings.multi_instance_desc").to_string()),
        ];
        let launch_page = SettingPage::new(t!("settings.page.launch")).group(
            SettingGroup::new()
                .title(t!("settings.group.behavior"))
                .items(launch_items),
        );

        let theme_options: Vec<(SharedString, SharedString)> = crate::theme::theme_names(cx)
            .into_iter()
            .map(|name| (name.clone(), name))
            .collect();

        let language_options: Vec<(SharedString, SharedString)> = language_options();

        let appearance_page = SettingPage::new(t!("settings.page.appearance")).group(
            SettingGroup::new().title(t!("settings.group.theme")).items(vec![
                SettingItem::new(
                    t!("settings.language"),
                    SettingField::dropdown(
                        language_options,
                        |cx| app_settings::get(cx).language.clone().into(),
                        patch_language,
                    ),
                )
                .description(t!("settings.language_desc").to_string()),
                SettingItem::new(
                    t!("settings.theme"),
                    SettingField::scrollable_dropdown(
                        theme_options,
                        |cx| app_settings::get(cx).theme_name.clone().into(),
                        patch_theme_name,
                    ),
                )
                .description(t!("settings.theme_desc").to_string()),
                // Sits right under the theme dropdown rather than in a titled row of
                // its own: both buttons are about where themes come from.
                SettingItem::render(|_, _, _| {
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            Button::new("open-themes-folder")
                                .icon(Icon::new(IconName::FolderOpen))
                                .label(t!("settings.open_themes_folder"))
                                .on_click(|_, _, cx| {
                                    open_folder(&crate::theme::themes_dir(), cx)
                                }),
                        )
                        .child(
                            Button::new("browse-themes")
                                .icon(Icon::new(IconName::ExternalLink))
                                .label(t!("settings.browse_themes"))
                                .on_click(|_, _, cx| {
                                    cx.open_url(
                                        "https://github.com/longbridge/gpui-component/tree/main/themes",
                                    )
                                }),
                        )
                }),
                SettingItem::new(
                    t!("settings.stars_background"),
                    SettingField::switch(
                        |cx| app_settings::get(cx).show_stars_background,
                        patch_show_stars_background,
                    ),
                )
                .description(t!("settings.stars_background_desc").to_string()),
                SettingItem::new(
                    t!("settings.scrollbars"),
                    SettingField::dropdown(
                        vec![
                            ("scrolling".into(), t!("settings.scrollbars_scrolling").to_string().into()),
                            ("hover".into(), t!("settings.scrollbars_hover").to_string().into()),
                            ("always".into(), t!("settings.scrollbars_always").to_string().into()),
                        ],
                        |cx| match app_settings::get(cx).scrollbar_visibility {
                            ScrollbarVisibility::Scrolling => "scrolling".into(),
                            ScrollbarVisibility::Hover => "hover".into(),
                            ScrollbarVisibility::Always => "always".into(),
                        },
                        patch_scrollbar_visibility,
                    ),
                )
                .description(t!("settings.scrollbars_desc").to_string()),
            ]));

        let bepinex_page = SettingPage::new(t!("settings.page.bepinex")).groups(vec![
            SettingGroup::new()
                .title(t!("settings.group.cache"))
                .items(vec![
                    SettingItem::new(
                        t!("settings.cache_downloads"),
                        SettingField::switch(
                            |cx| app_settings::get(cx).cache_bepinex,
                            |value, cx| app_settings::update(cx, |s| s.cache_bepinex = value),
                        ),
                    )
                    .description(t!("settings.cache_downloads_desc").to_string()),
                    cache_item(BinaryArch::X64, t!("settings.cache.x64").into()),
                    cache_item(BinaryArch::X86, t!("settings.cache.x86").into()),
                ]),
            SettingGroup::new()
                .title(t!("settings.group.download_urls"))
                .description(t!("settings.download_urls_desc"))
                .items(vec![
                    stacked_item(
                        t!("settings.bepinex_x64_url"),
                        SettingField::input(
                            |cx| app_settings::get(cx).bepinex_url_x64.clone().into(),
                            |value, cx| {
                                app_settings::update(cx, |s| s.bepinex_url_x64 = value.to_string())
                            },
                        ),
                    ),
                    stacked_item(
                        t!("settings.bepinex_x86_url"),
                        SettingField::input(
                            |cx| app_settings::get(cx).bepinex_url_x86.clone().into(),
                            |value, cx| {
                                app_settings::update(cx, |s| s.bepinex_url_x86 = value.to_string())
                            },
                        ),
                    ),
                ]),
        ]);

        #[cfg(unix)]
        let linux_page = {
            let kind = app_settings::get(cx).game.linux_runner_kind.clone();

            let auto_detect = SettingItem::new(
                t!("settings.auto_detect"),
                SettingField::render(|_, _, _| {
                    Button::new("detect-linux-runtime")
                        .icon(Icon::new(AppIcon::Compass))
                        .label(t!("settings.linux.auto_detect"))
                        .on_click(|_, window, cx| detect_linux_runtime(window, cx))
                }),
            )
            .description(t!("settings.linux.auto_detect_desc").to_string());

            let runner = SettingItem::new(
                t!("settings.linux.runner"),
                SettingField::dropdown(
                    vec![
                        ("steam".into(), "Steam".into()),
                        ("proton".into(), "Proton".into()),
                        ("wine".into(), "Wine".into()),
                    ],
                    |cx| match app_settings::get(cx).game.linux_runner_kind {
                        LinuxRunnerKind::Wine => "wine".into(),
                        LinuxRunnerKind::Proton => "proton".into(),
                        LinuxRunnerKind::Steam => "steam".into(),
                    },
                    patch_linux_runner_kind,
                ),
            )
            .description(t!("settings.linux.runner_desc").to_string());

            let runner_binary = stacked_item(
                t!("settings.linux.runner_binary"),
                path_field(
                    "linux-runner-binary",
                    false,
                    |cx| {
                        app_settings::get(cx)
                            .game
                            .linux_runner_binary
                            .clone()
                            .into()
                    },
                    |value, cx| {
                        app_settings::update(cx, |s| s.game.linux_runner_binary = value.to_string())
                    },
                ),
            );

            let wine_prefix = stacked_item(
                t!("settings.linux.wine_prefix"),
                path_field(
                    "linux-wine-prefix",
                    true,
                    |cx| app_settings::get(cx).game.linux_wine_prefix.clone().into(),
                    |value, cx| {
                        app_settings::update(cx, |s| s.game.linux_wine_prefix = value.to_string())
                    },
                ),
            );

            let wine_region_info = stacked_item(
                t!("settings.linux.region_info_path"),
                path_field(
                    "linux-wine-region-info",
                    false,
                    |cx| {
                        app_settings::get(cx)
                            .linux_wine_region_info_path
                            .clone()
                            .into()
                    },
                    |value, cx| {
                        app_settings::update(cx, |s| {
                            s.linux_wine_region_info_path = value.to_string()
                        })
                    },
                ),
            )
            .description(t!("settings.linux.region_info_desc").to_string());

            let proton_compat = stacked_item(
                t!("settings.linux.proton_compat"),
                path_field(
                    "linux-proton-compat",
                    true,
                    |cx| {
                        app_settings::get(cx)
                            .game
                            .linux_proton_compat_data_path
                            .clone()
                            .into()
                    },
                    |value, cx| {
                        app_settings::update(cx, |s| {
                            s.game.linux_proton_compat_data_path = value.to_string()
                        })
                    },
                ),
            )
            .description(t!("settings.linux.proton_compat_desc").to_string());

            let steam_run = SettingItem::new(
                t!("settings.linux.steam_run"),
                SettingField::switch(
                    |cx| app_settings::get(cx).game.linux_proton_use_steam_run,
                    |value, cx| {
                        app_settings::update(cx, |s| s.game.linux_proton_use_steam_run = value)
                    },
                ),
            )
            .description(t!("settings.linux.steam_run_desc").to_string());

            // Only show the fields the selected runner actually uses.
            let items = match kind {
                LinuxRunnerKind::Steam => vec![auto_detect, runner, proton_compat],
                LinuxRunnerKind::Wine => {
                    vec![
                        auto_detect,
                        runner,
                        runner_binary,
                        wine_prefix,
                        wine_region_info,
                    ]
                }
                LinuxRunnerKind::Proton => {
                    vec![auto_detect, runner, runner_binary, proton_compat, steam_run]
                }
            };

            SettingPage::new(t!("settings.page.linux")).group(
                SettingGroup::new()
                    .title(t!("settings.group.runner"))
                    .description(t!("settings.group.runner_desc"))
                    .items(items),
            )
        };

        // Only Windows can install an update in place (see `update_service`),
        // so only Windows gets the group (added to the About page below).
        let updates_group = SettingGroup::new()
            .title(t!("settings.group.updates"))
            .items(vec![
                SettingItem::new(
                    t!("settings.release_channel"),
                    SettingField::dropdown(
                        vec![
                            (
                                "stable".into(),
                                t!("settings.channel_stable").to_string().into(),
                            ),
                            (
                                "nightly".into(),
                                t!("settings.channel_nightly").to_string().into(),
                            ),
                        ],
                        |cx| match app_settings::get(cx).release_channel {
                            ReleaseChannel::Stable => "stable".into(),
                            ReleaseChannel::Nightly => "nightly".into(),
                        },
                        patch_release_channel,
                    ),
                )
                .description(t!("settings.release_channel_desc").to_string()),
                SettingItem::new(
                    t!("settings.check_for_updates"),
                    SettingField::render(|_, _, cx| render_update_controls(cx)),
                )
                .description(update_status(updater::state(cx))),
            ]);

        let about_page =
            SettingPage::new(t!("settings.page.about")).group(SettingGroup::new().items(vec![
                SettingItem::render(|_, _window, cx| {
                    let theme = cx.theme().clone();
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::BOLD)
                                        .child("Starlight PC"),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded_full()
                                        .bg(theme.accent)
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(concat!("v", env!("CARGO_PKG_VERSION"))),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("♡ 2026 All Of Us Mods")
                                .child("|")
                                .child(
                                    div()
                                        .id("about-license-link")
                                        .cursor_pointer()
                                        .hover(|s| s.text_color(theme.foreground))
                                        .child(t!("settings.license"))
                                        .on_click(|_, _, cx| {
                                            cx.open_url("https://www.gnu.org/licenses/gpl-3.0.html")
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    Button::new("about-view-source")
                                        .icon(Icon::new(IconName::ExternalLink))
                                        .label(t!("settings.view_source"))
                                        .on_click(|_, _, cx| {
                                            cx.open_url(
                                                "https://github.com/All-Of-Us-Mods/Starlight-PC",
                                            )
                                        }),
                                )
                                .child(
                                    Button::new("about-open-data")
                                        .icon(Icon::new(IconName::FolderOpen))
                                        .label(t!("settings.open_data_folder"))
                                        .on_click(|_, _, cx| open_data_folder(cx)),
                                ),
                        )
                }),
            ]));

        let about_page = if cfg!(windows) {
            about_page.group(updates_group)
        } else {
            about_page
        };

        crate::views::page_root("settings-page", &theme)
            .overflow_y_scrollbar()
            .gap_4()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child(t!("nav.settings")),
            )
            .child(
                Settings::new("starlight-settings")
                    .sidebar_width(px(190.0))
                    .pages({
                        #[cfg_attr(not(unix), allow(unused_mut))]
                        let mut pages = vec![game_page, launch_page, appearance_page, bepinex_page];
                        #[cfg(unix)]
                        pages.push(linux_page);
                        pages.push(about_page);
                        pages
                    }),
            )
    }
}
