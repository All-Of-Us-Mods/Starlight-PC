use gpui::*;
use log::warn;

use crate::backend::services::core_service::{
    self, AppSettings, AppSettingsPatch, GamePlatform,
};
use crate::theme::{self, ThemeExt};

pub struct SettingsView {
    state: LoadState,
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

fn section(title: &'static str, children: Vec<AnyElement>, theme: &crate::theme::Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .p_4()
        .rounded_lg()
        .bg(theme.sidebar_background)
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .pb_2()
                .child(title),
        )
        .children(children)
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
                    "Game",
                    vec![self
                        .render_platform_selector(platform, &theme, cx)
                        .into_any_element()],
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
                    "BepInEx",
                    vec![self
                        .render_toggle(
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
                        .into_any_element()],
                    &theme,
                );
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(game_section)
                    .child(launch_section)
                    .child(bepinex_section)
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
            .child(body)
    }
}
