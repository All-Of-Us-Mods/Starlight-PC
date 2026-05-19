use std::rc::Rc;

use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    AxisExt as _, Icon, IconName, Sizable as _, WindowExt,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
    notification::Notification,
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
};
use log::warn;

use crate::backend::services::{
    bepinex_service,
    core_service::{self, AppSettingsPatch, GamePlatform, LinuxRunnerKind},
    finder_service,
};
use crate::settings as app_settings;
use crate::theme::{self, ThemeExt};
use crate::ui::icon::AppIcon;

pub struct SettingsView;

impl SettingsView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.observe_global::<app_settings::SettingsGlobal>(|_, cx| cx.notify())
            .detach();
        Self
    }
}

// ---------- patch helpers (used by setter closures) ----------

fn patch_among_us_path(value: SharedString, cx: &mut App) {
    app_settings::update(
        cx,
        AppSettingsPatch {
            among_us_path: Some(value.to_string()),
            ..Default::default()
        },
    );
}

fn patch_close_on_launch(value: bool, cx: &mut App) {
    app_settings::update(
        cx,
        AppSettingsPatch {
            close_on_launch: Some(value),
            ..Default::default()
        },
    );
}

fn patch_multi_instance(value: bool, cx: &mut App) {
    app_settings::update(
        cx,
        AppSettingsPatch {
            allow_multi_instance_launch: Some(value),
            ..Default::default()
        },
    );
}

fn patch_cache_bepinex(value: bool, cx: &mut App) {
    app_settings::update(
        cx,
        AppSettingsPatch {
            cache_bepinex: Some(value),
            ..Default::default()
        },
    );
}

fn patch_platform(value: SharedString, cx: &mut App) {
    let platform = match value.as_ref() {
        "epic" => GamePlatform::Epic,
        "xbox" => GamePlatform::Xbox,
        _ => GamePlatform::Steam,
    };
    app_settings::update(
        cx,
        AppSettingsPatch {
            game_platform: Some(platform),
            ..Default::default()
        },
    );
}

fn patch_bepinex_url_x64(value: SharedString, cx: &mut App) {
    app_settings::update(
        cx,
        AppSettingsPatch {
            bepinex_url_x64: Some(value.to_string()),
            ..Default::default()
        },
    );
}

fn patch_bepinex_url_x86(value: SharedString, cx: &mut App) {
    app_settings::update(
        cx,
        AppSettingsPatch {
            bepinex_url_x86: Some(value.to_string()),
            ..Default::default()
        },
    );
}

fn patch_linux_runner_kind(value: SharedString, cx: &mut App) {
    let kind = match value.as_ref() {
        "wine" => LinuxRunnerKind::Wine,
        _ => LinuxRunnerKind::Proton,
    };
    app_settings::update(
        cx,
        AppSettingsPatch {
            linux_runner_kind: Some(kind),
            ..Default::default()
        },
    );
}

fn patch_linux_runner_binary(value: SharedString, cx: &mut App) {
    app_settings::update(
        cx,
        AppSettingsPatch {
            linux_runner_binary: Some(value.to_string()),
            ..Default::default()
        },
    );
}

fn patch_linux_wine_prefix(value: SharedString, cx: &mut App) {
    app_settings::update(
        cx,
        AppSettingsPatch {
            linux_wine_prefix: Some(value.to_string()),
            ..Default::default()
        },
    );
}

fn patch_linux_proton_compat_data_path(value: SharedString, cx: &mut App) {
    app_settings::update(
        cx,
        AppSettingsPatch {
            linux_proton_compat_data_path: Some(value.to_string()),
            ..Default::default()
        },
    );
}

// ---------- path input field (Input + Browse button, two-way bound) ----------

struct PathFieldState {
    input: Entity<InputState>,
    _sub: Subscription,
}

/// File-path setting field. The input mirrors the global in real time (so an
/// external write like Auto-detect updates the visible text), edits write back
/// through `set`, and the Browse button opens the platform file picker.
fn path_field(
    key: &'static str,
    directories_only: bool,
    get: fn(&App) -> SharedString,
    set: fn(SharedString, &mut App),
) -> SettingField<SharedString> {
    SettingField::render(move |options, window, cx| {
        let value = get(cx);

        let state_key: SharedString = format!(
            "path-field-{}-{}-{}-{}",
            key, options.page_ix, options.group_ix, options.item_ix
        )
        .into();

        let value_for_init = value.clone();
        let state = window.use_keyed_state(state_key, cx, move |window, cx| {
            let input =
                cx.new(|cx| InputState::new(window, cx).default_value(value_for_init.clone()));
            let _sub = cx.subscribe(&input, move |_, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let v = input.read(cx).value();
                    set(v, cx);
                }
            });
            PathFieldState { input, _sub }
        });

        let input_entity = state.read(cx).input.clone();
        if input_entity.read(cx).value() != value {
            let val = value.clone();
            input_entity.update(cx, |s, cx| s.set_value(val, window, cx));
        }

        let prompt: SharedString = if directories_only {
            "Select folder".into()
        } else {
            "Select file".into()
        };
        let button_id: SharedString = format!(
            "path-browse-{}-{}-{}-{}",
            key, options.page_ix, options.group_ix, options.item_ix
        )
        .into();
        let setter: Rc<dyn Fn(SharedString, &mut App)> = Rc::new(move |v, cx| set(v, cx));

        let input_el = Input::new(&input_entity).with_size(options.size).map(|this| {
            if options.layout.is_horizontal() {
                this.w_64()
            } else {
                this.w_full()
            }
        });

        div().flex().gap_2().child(input_el).child(
            Button::new(button_id)
                .icon(Icon::new(IconName::FolderOpen))
                .label("Browse")
                .with_size(options.size)
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

fn detect_linux_runtime(window: &mut Window, cx: &mut App) {
    let among_us_path = app_settings::get(cx).among_us_path.clone();
    let path_arg = (!among_us_path.trim().is_empty()).then(|| among_us_path);
    match finder_service::detect_linux_runner(path_arg) {
        Ok(detection) => {
            let kind = match detection.runner_kind.as_str() {
                "wine" => LinuxRunnerKind::Wine,
                _ => LinuxRunnerKind::Proton,
            };
            app_settings::update(
                cx,
                AppSettingsPatch {
                    linux_runner_kind: Some(kind),
                    linux_runner_binary: Some(detection.runner_binary.unwrap_or_default()),
                    linux_wine_prefix: Some(detection.wine_prefix.unwrap_or_default()),
                    linux_proton_compat_data_path: Some(
                        detection.proton_compat_data_path.unwrap_or_default(),
                    ),
                    linux_proton_steam_client_path: Some(
                        detection.proton_steam_client_path.unwrap_or_default(),
                    ),
                    linux_proton_use_steam_run: Some(detection.proton_use_steam_run),
                    ..Default::default()
                },
            );
            window.push_notification(
                Notification::success("Linux runtime detected"),
                cx,
            );
        }
        Err(e) => {
            warn!("detect_linux_runner failed: {e}");
            window.push_notification(
                Notification::error(format!("Detection failed: {e}")),
                cx,
            );
        }
    }
}

fn detect_among_us(window: &mut Window, cx: &mut App) {
    match finder_service::detect_among_us_installation() {
        Ok(Some(path)) => {
            app_settings::update(
                cx,
                AppSettingsPatch {
                    among_us_path: Some(path.clone()),
                    ..Default::default()
                },
            );
            window.push_notification(
                Notification::success(format!("Among Us detected at {path}")),
                cx,
            );
        }
        Ok(None) => {
            window.push_notification(
                Notification::warning("Could not auto-detect Among Us installation"),
                cx,
            );
        }
        Err(e) => {
            warn!("detect_among_us failed: {e}");
            window.push_notification(
                Notification::error(format!("Detection failed: {e}")),
                cx,
            );
        }
    }
}

fn download_bepinex_cache(arch: &'static str, window: &mut Window, cx: &mut App) {
    let settings = app_settings::get(cx).clone();
    let cache_path = match core_service::get_bepinex_cache_path(arch) {
        Ok(p) => p,
        Err(e) => {
            window.push_notification(
                Notification::error(format!("Cache path resolution failed: {e}")),
                cx,
            );
            return;
        }
    };
    let url = if arch == "x64" {
        settings.bepinex_url_x64
    } else {
        settings.bepinex_url_x86
    };
    let arch_owned = arch.to_string();
    cx.background_executor()
        .spawn(async move {
            if let Err(e) =
                bepinex_service::download_bepinex_to_cache(url, cache_path, arch_owned.clone())
            {
                warn!("BepInEx cache download ({arch_owned}) failed: {e}");
            }
        })
        .detach();
    window.push_notification(
        Notification::info(format!("Downloading BepInEx {arch}…")),
        cx,
    );
}

fn clear_bepinex_cache(arch: &'static str, window: &mut Window, cx: &mut App) {
    match core_service::get_bepinex_cache_path(arch) {
        Ok(path) => match bepinex_service::clear_cache(path) {
            Ok(()) => window.push_notification(
                Notification::success(format!("Cleared BepInEx {arch} cache")),
                cx,
            ),
            Err(e) => {
                warn!("clear_bepinex_cache failed: {e}");
                window.push_notification(Notification::error(format!("Clear failed: {e}")), cx);
            }
        },
        Err(e) => {
            window.push_notification(Notification::error(format!("Cache path: {e}")), cx);
        }
    }
}

// ---------- view ----------

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let game_page = SettingPage::new("Game").default_open(true).groups(vec![
            SettingGroup::new().title("Installation").items(vec![
                SettingItem::new(
                    "Among Us path",
                    path_field(
                        "among-us",
                        true,
                        |cx| app_settings::get(cx).among_us_path.clone().into(),
                        patch_among_us_path,
                    ),
                )
                .description("Folder containing Among Us.exe."),
                SettingItem::new(
                    "Auto-detect",
                    SettingField::render(|_, _, _| {
                        Button::new("detect-among-us")
                            .icon(Icon::new(AppIcon::Compass))
                            .label("Auto-detect Among Us")
                            .on_click(|_, window, cx| detect_among_us(window, cx))
                    }),
                )
                .description("Search known install locations and set the path above."),
            ]),
            SettingGroup::new().title("Platform").items(vec![
                SettingItem::new(
                    "Game platform",
                    SettingField::dropdown(
                        vec![
                            ("steam".into(), "Steam".into()),
                            ("epic".into(), "Epic".into()),
                            ("xbox".into(), "Xbox".into()),
                        ],
                        |cx| match app_settings::get(cx).game_platform {
                            GamePlatform::Steam => "steam".into(),
                            GamePlatform::Epic => "epic".into(),
                            GamePlatform::Xbox => "xbox".into(),
                        },
                        patch_platform,
                    ),
                )
                .description("Which storefront the game was installed from."),
            ]),
        ]);

        let launch_page = SettingPage::new("Launch").group(
            SettingGroup::new().title("Behavior").items(vec![
                SettingItem::new(
                    "Close Starlight when launching",
                    SettingField::switch(
                        |cx| app_settings::get(cx).close_on_launch,
                        patch_close_on_launch,
                    ),
                )
                .description("Quit the app after starting the game."),
                SettingItem::new(
                    "Allow multiple instances",
                    SettingField::switch(
                        |cx| app_settings::get(cx).allow_multi_instance_launch,
                        patch_multi_instance,
                    ),
                )
                .description("Permit launching more than one game window at a time."),
            ]),
        );

        let bepinex_page = SettingPage::new("BepInEx").groups(vec![
            SettingGroup::new().title("Cache").items(vec![
                SettingItem::new(
                    "Cache BepInEx downloads",
                    SettingField::switch(
                        |cx| app_settings::get(cx).cache_bepinex,
                        patch_cache_bepinex,
                    ),
                )
                .description("Reuse cached archives across profile installs."),
                SettingItem::new(
                    "x64 cache",
                    SettingField::render(|_, _, _| {
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("cache-x64")
                                    .icon(Icon::new(AppIcon::Download))
                                    .label("Download")
                                    .on_click(|_, window, cx| {
                                        download_bepinex_cache("x64", window, cx)
                                    }),
                            )
                            .child(
                                Button::new("clear-x64")
                                    .danger()
                                    .icon(Icon::new(IconName::Delete))
                                    .label("Clear")
                                    .on_click(|_, window, cx| {
                                        clear_bepinex_cache("x64", window, cx)
                                    }),
                            )
                    }),
                ),
                SettingItem::new(
                    "x86 cache",
                    SettingField::render(|_, _, _| {
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("cache-x86")
                                    .icon(Icon::new(AppIcon::Download))
                                    .label("Download")
                                    .on_click(|_, window, cx| {
                                        download_bepinex_cache("x86", window, cx)
                                    }),
                            )
                            .child(
                                Button::new("clear-x86")
                                    .danger()
                                    .icon(Icon::new(IconName::Delete))
                                    .label("Clear")
                                    .on_click(|_, window, cx| {
                                        clear_bepinex_cache("x86", window, cx)
                                    }),
                            )
                    }),
                ),
            ]),
            SettingGroup::new()
                .title("Download URLs")
                .description("Override the default release archive locations.")
                .items(vec![
                    SettingItem::new(
                        "BepInEx x64 URL",
                        SettingField::input(
                            |cx| app_settings::get(cx).bepinex_url_x64.clone().into(),
                            patch_bepinex_url_x64,
                        ),
                    ),
                    SettingItem::new(
                        "BepInEx x86 URL",
                        SettingField::input(
                            |cx| app_settings::get(cx).bepinex_url_x86.clone().into(),
                            patch_bepinex_url_x86,
                        ),
                    ),
                ]),
        ]);

        let linux_page = SettingPage::new("Linux runtime").group(
            SettingGroup::new()
                .title("Wine / Proton")
                .description("Used when launching the game on Linux.")
                .items(vec![
                    SettingItem::new(
                        "Auto-detect",
                        SettingField::render(|_, _, _| {
                            Button::new("detect-linux-runtime")
                                .icon(Icon::new(AppIcon::Compass))
                                .label("Auto-detect Linux runtime")
                                .on_click(|_, window, cx| detect_linux_runtime(window, cx))
                        }),
                    )
                    .description("Probe Steam/Proton + Wine prefixes from the Among Us path."),
                    SettingItem::new(
                        "Runner",
                        SettingField::dropdown(
                            vec![
                                ("proton".into(), "Proton".into()),
                                ("wine".into(), "Wine".into()),
                            ],
                            |cx| match app_settings::get(cx).linux_runner_kind {
                                LinuxRunnerKind::Wine => "wine".into(),
                                LinuxRunnerKind::Proton => "proton".into(),
                            },
                            patch_linux_runner_kind,
                        ),
                    ),
                    SettingItem::new(
                        "Runner binary",
                        path_field(
                            "linux-runner-binary",
                            false,
                            |cx| app_settings::get(cx).linux_runner_binary.clone().into(),
                            patch_linux_runner_binary,
                        ),
                    ),
                    SettingItem::new(
                        "Wine prefix",
                        path_field(
                            "linux-wine-prefix",
                            true,
                            |cx| app_settings::get(cx).linux_wine_prefix.clone().into(),
                            patch_linux_wine_prefix,
                        ),
                    ),
                    SettingItem::new(
                        "Proton compat data path",
                        path_field(
                            "linux-proton-compat",
                            true,
                            |cx| {
                                app_settings::get(cx)
                                    .linux_proton_compat_data_path
                                    .clone()
                                    .into()
                            },
                            patch_linux_proton_compat_data_path,
                        ),
                    ),
                ]),
        );

        div()
            .id("settings-page")
            .flex()
            .flex_col()
            .size_full()
            .overflow_y_scroll()
            .font_family(theme::FONT_FAMILY)
            .text_color(theme.text)
            .text_size(px(14.0))
            .p_8()
            .pt(px(48.0))
            .gap_4()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Settings"),
            )
            .child(
                Settings::new("starlight-settings")
                    .pages(vec![game_page, launch_page, bepinex_page, linux_page]),
            )
    }
}
