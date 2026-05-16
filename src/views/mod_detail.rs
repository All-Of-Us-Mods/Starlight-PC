use chrono::{DateTime, Local};
use gpui::*;

use crate::backend::api::{self, ModResponse, ModVersion, ModVersionInfo};
use crate::theme::{self, ThemeExt};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::skeleton::Skeleton;
use gpui_component::tag::Tag;
use gpui_component::{Icon, IconName};
use crate::ui::mod_card::format_count;

#[derive(Clone, Debug)]
pub enum ModDetailEvent {
    Close,
}

impl EventEmitter<ModDetailEvent> for ModDetailView {}

pub struct ModDetailView {
    state: LoadState,
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

impl ModDetailView {
    pub fn new(mod_id: String, cx: &mut Context<Self>) -> Self {
        let view = Self {
            state: LoadState::Loading,
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
                    Ok::<ModDetailData, crate::backend::error::AppError>(ModDetailData {
                        mod_info,
                        versions,
                        version_info,
                    })
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.state = match result {
                    Ok(m) => LoadState::Loaded(m),
                    Err(e) => LoadState::Failed(e.to_string()),
                };
                cx.notify();
            });
        })
        .detach();
        view
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
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(section_label("Version", &theme))
                            .child(if let Some(version) = data.versions.first() {
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .rounded_lg()
                                    .bg(theme.hover)
                                    .border_1()
                                    .border_color(theme.border)
                                    .px_3()
                                    .py_2()
                                    .child(format!("Latest: {}", version.version))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.text_muted)
                                            .child(format_date(version.created_at)),
                                    )
                                    .into_any_element()
                            } else {
                                div()
                                    .text_sm()
                                    .text_color(theme.text_muted)
                                    .child("No versions available")
                                    .into_any_element()
                            }),
                    )
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
                    .children(data.version_info.as_ref().and_then(|version| {
                        if version.dependencies.is_empty() {
                            None
                        } else {
                            Some(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(section_label("Dependencies", &theme))
                                    .children(version.dependencies.iter().map(|dep| {
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .rounded_lg()
                                            .bg(theme.hover)
                                            .px_3()
                                            .py_2()
                                            .child(dep.name.clone())
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .text_xs()
                                                    .text_color(theme.text_muted)
                                                    .child(format!("v{}", dep.version_constraint))
                                                    .child(dep.dependency_type.clone()),
                                            )
                                    })),
                            )
                        }
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
