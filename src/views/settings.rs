use gpui::*;
use log::warn;

use crate::backend::services::{
    bepinex_service,
    core_service::{self, AppSettings, AppSettingsPatch, GamePlatform},
    finder_service,
};
use crate::theme::{self, ThemeExt};
use crate::ui::icon::AppIcon;
use gpui_component::{Icon, IconName};

pub struct SettingsView {
    state: LoadState,
    message: Option<String>,
}

enum LoadState {
    Loading,
    Loaded(AppSettings),
    Failed(String),
}

impl SettingsView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let view = Self {
            state: LoadState::Loading,
            message: None,
        };
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { core_service::get_settings() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.state = match result {
                    Ok(s) => LoadState::Loaded(s),
                    Err(e) => LoadState::Failed(e.to_string()),
                };
                cx.notify();
            });
        })
        .detach();
        view
    }

    fn apply_patch(&mut self, patch: AppSettingsPatch, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { core_service::update_settings(patch) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(s) => this.state = LoadState::Loaded(s),
                    Err(e) => warn!("update_settings failed: {e}"),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn set_message(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        self.message = Some(message.into());
        cx.notify();
    }

    fn detect_among_us_path(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { finder_service::detect_among_us_installation() })
                .await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(Some(path)) => {
                    this.apply_patch(
                        AppSettingsPatch {
                            among_us_path: Some(path.clone()),
                            ..Default::default()
                        },
                        cx,
                    );
                    this.message = Some(format!("Detected Among Us at {path}"));
                }
                Ok(None) => this.set_message("Could not auto-detect Among Us", cx),
                Err(e) => this.set_message(format!("Detection failed: {e}"), cx),
            });
        })
        .detach();
    }

    fn download_bepinex_cache(&mut self, architecture: &'static str, cx: &mut Context<Self>) {
        let settings = match &self.state {
            LoadState::Loaded(settings) => settings.clone(),
            _ => return,
        };
        let url = if architecture == "x86" {
            settings.bepinex_url_x86
        } else {
            settings.bepinex_url_x64
        };
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let cache_path = core_service::get_bepinex_cache_path(architecture)?;
                    bepinex_service::download_bepinex_to_cache(
                        url,
                        cache_path,
                        architecture.to_string(),
                    )
                })
                .await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(_) => this.set_message(format!("Cached BepInEx {architecture}"), cx),
                Err(e) => this.set_message(format!("Cache download failed: {e}"), cx),
            });
        })
        .detach();
    }

    fn clear_bepinex_cache(&mut self, architecture: &'static str, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let path = core_service::get_bepinex_cache_path(architecture)?;
                    bepinex_service::clear_cache(path)
                })
                .await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(_) => this.set_message(format!("Cleared BepInEx {architecture} cache"), cx),
                Err(e) => this.set_message(format!("Clear cache failed: {e}"), cx),
            });
        })
        .detach();
    }

    fn action_button(
        &self,
        id: &'static str,
        label: &'static str,
        leading: Option<Icon>,
        theme: &crate::theme::Theme,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1p5()
            .rounded_md()
            .bg(theme.hover)
            .cursor_pointer()
            .hover(|s| s.opacity(0.85))
            .children(leading)
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| on_click(this, cx)))
    }

    fn render_toggle(
        &self,
        id: &'static str,
        label: &'static str,
        value: bool,
        theme: &crate::theme::Theme,
        on_toggle: impl Fn(&mut Self, bool, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let knob_color = if value { theme.primary } else { theme.hover };
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_between()
            .py_2()
            .cursor_pointer()
            .child(div().child(label))
            .child(
                div()
                    .w(px(40.0))
                    .h(px(22.0))
                    .rounded_full()
                    .bg(knob_color)
                    .p(px(2.0))
                    .child(
                        div()
                            .w(px(18.0))
                            .h(px(18.0))
                            .rounded_full()
                            .bg(theme.text)
                            .ml(if value { px(18.0) } else { px(0.0) }),
                    ),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                on_toggle(this, !value, cx);
            }))
    }

    fn render_platform_selector(
        &self,
        current: GamePlatform,
        theme: &crate::theme::Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let make_btn = |id: &'static str, label: &'static str, platform: GamePlatform| {
            let active = current == platform;
            div()
                .id(id)
                .px_3()
                .py_1p5()
                .rounded_md()
                .bg(if active { theme.primary } else { theme.hover })
                .cursor_pointer()
                .child(label)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.apply_patch(
                        AppSettingsPatch {
                            game_platform: Some(platform),
                            ..Default::default()
                        },
                        cx,
                    );
                }))
        };

        div()
            .flex()
            .items_center()
            .justify_between()
            .py_2()
            .child(div().child("Game platform"))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(make_btn("plat-steam", "Steam", GamePlatform::Steam))
                    .child(make_btn("plat-epic", "Epic", GamePlatform::Epic))
                    .child(make_btn("plat-xbox", "Xbox", GamePlatform::Xbox)),
            )
    }
}

fn section(
    title: &'static str,
    children: Vec<AnyElement>,
    theme: &crate::theme::Theme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .p_4()
        .rounded_lg()
        .bg(theme.sidebar_background)
        .border_1()
        .border_color(theme.border)
        .child(div().font_weight(FontWeight::SEMIBOLD).pb_2().child(title))
        .children(children)
}

fn readonly_row(
    label: &'static str,
    value: String,
    theme: &crate::theme::Theme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .py_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(label),
        )
        .child(
            div()
                .rounded_md()
                .bg(theme.hover)
                .px_3()
                .py_2()
                .text_sm()
                .text_color(theme.text_muted)
                .child(if value.is_empty() {
                    "Not configured".to_string()
                } else {
                    value
                }),
        )
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let body: AnyElement = match &self.state {
            LoadState::Loading => div().child("Loading settings…").into_any_element(),
            LoadState::Failed(e) => div()
                .text_color(rgb(0xef4444))
                .child(format!("Failed: {e}"))
                .into_any_element(),
            LoadState::Loaded(settings) => {
                let close = settings.close_on_launch;
                let multi = settings.allow_multi_instance_launch;
                let cache = settings.cache_bepinex;
                let platform = settings.game_platform;
                let game_section = section(
                    "Game Configuration",
                    vec![
                        readonly_row("Among Us installation path", settings.among_us_path.clone(), &theme)
                            .into_any_element(),
                        self.action_button(
                            "detect-among-us",
                            "Auto-detect Among Us",
                            Some(Icon::new(AppIcon::Compass)),
                            &theme,
                            |this, cx| this.detect_among_us_path(cx),
                            cx,
                        )
                        .into_any_element(),
                        self.render_platform_selector(platform, &theme, cx)
                            .into_any_element(),
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(match platform {
                                GamePlatform::Steam => "Steam launches through the local Among Us install.",
                                GamePlatform::Epic => "Epic Games launch requires an authenticated Epic account in the full app flow.",
                                GamePlatform::Xbox => "Xbox launch uses the detected Xbox app id when available.",
                            })
                            .into_any_element(),
                    ],
                    &theme,
                );
                let launch_section = section(
                    "Launch",
                    vec![
                        self.render_toggle(
                            "close-on-launch",
                            "Close Starlight when launching the game",
                            close,
                            &theme,
                            move |this, v, cx| {
                                this.apply_patch(
                                    AppSettingsPatch {
                                        close_on_launch: Some(v),
                                        ..Default::default()
                                    },
                                    cx,
                                );
                            },
                            cx,
                        )
                        .into_any_element(),
                        self.render_toggle(
                            "multi-instance",
                            "Allow multiple instances of the game",
                            multi,
                            &theme,
                            move |this, v, cx| {
                                this.apply_patch(
                                    AppSettingsPatch {
                                        allow_multi_instance_launch: Some(v),
                                        ..Default::default()
                                    },
                                    cx,
                                );
                            },
                            cx,
                        )
                        .into_any_element(),
                    ],
                    &theme,
                );
                let bepinex_section = section(
                    "BepInEx Configuration",
                    vec![
                        readonly_row(
                            "BepInEx x64 download URL",
                            settings.bepinex_url_x64.clone(),
                            &theme,
                        )
                        .into_any_element(),
                        readonly_row(
                            "BepInEx x86 download URL",
                            settings.bepinex_url_x86.clone(),
                            &theme,
                        )
                        .into_any_element(),
                        self.render_toggle(
                            "cache-bepinex",
                            "Cache BepInEx downloads",
                            cache,
                            &theme,
                            move |this, v, cx| {
                                this.apply_patch(
                                    AppSettingsPatch {
                                        cache_bepinex: Some(v),
                                        ..Default::default()
                                    },
                                    cx,
                                );
                            },
                            cx,
                        )
                        .into_any_element(),
                        div()
                            .flex()
                            .gap_2()
                            .child(self.action_button(
                                "cache-x64",
                                "Cache x64",
                                Some(Icon::new(AppIcon::Download)),
                                &theme,
                                |this, cx| this.download_bepinex_cache("x64", cx),
                                cx,
                            ))
                            .child(self.action_button(
                                "cache-x86",
                                "Cache x86",
                                Some(Icon::new(AppIcon::Download)),
                                &theme,
                                |this, cx| this.download_bepinex_cache("x86", cx),
                                cx,
                            ))
                            .child(self.action_button(
                                "clear-x64",
                                "Clear x64",
                                Some(Icon::new(IconName::Delete)),
                                &theme,
                                |this, cx| this.clear_bepinex_cache("x64", cx),
                                cx,
                            ))
                            .child(self.action_button(
                                "clear-x86",
                                "Clear x86",
                                Some(Icon::new(IconName::Delete)),
                                &theme,
                                |this, cx| this.clear_bepinex_cache("x86", cx),
                                cx,
                            ))
                            .into_any_element(),
                    ],
                    &theme,
                );
                let platform_section = section(
                    "Platform Runtime",
                    vec![
                        readonly_row(
                            "Linux runner",
                            format!("{:?}", settings.linux_runner_kind),
                            &theme,
                        )
                        .into_any_element(),
                        readonly_row(
                            "Runner binary",
                            settings.linux_runner_binary.clone(),
                            &theme,
                        )
                        .into_any_element(),
                        readonly_row("Wine prefix", settings.linux_wine_prefix.clone(), &theme)
                            .into_any_element(),
                        readonly_row(
                            "Proton compat data path",
                            settings.linux_proton_compat_data_path.clone(),
                            &theme,
                        )
                        .into_any_element(),
                    ],
                    &theme,
                );
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(game_section)
                    .child(launch_section)
                    .child(bepinex_section)
                    .child(platform_section)
                    .into_any_element()
            }
        };

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
            .children(self.message.clone().map(|message| {
                div()
                    .rounded_md()
                    .bg(theme.hover)
                    .border_1()
                    .border_color(theme.border)
                    .p_3()
                    .text_sm()
                    .child(message)
            }))
            .child(body)
    }
}
