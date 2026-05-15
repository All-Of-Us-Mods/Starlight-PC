use gpui::*;
use log::warn;

use crate::backend::services::profile_service::{self, ProfileEntry};
use crate::theme::ThemeExt;

pub struct LibraryDetailView {
    profile_id: String,
    state: LoadState,
}

enum LoadState {
    Loading,
    Loaded(ProfileEntry),
    NotFound,
    Failed(String),
}

impl LibraryDetailView {
    pub fn new(profile_id: String, cx: &mut Context<Self>) -> Self {
        let view = Self {
            profile_id: profile_id.clone(),
            state: LoadState::Loading,
        };
        cx.spawn(async move |this, cx| {
            let id_for_task = profile_id.clone();
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::get_profile_by_id(&id_for_task) })
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
        view
    }

    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    fn install_bepinex(&mut self, cx: &mut Context<Self>) {
        let id = self.profile_id.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::install_bepinex_for_profile(&id) })
                .await;
            if let Err(e) = result {
                warn!("install_bepinex failed: {e}");
            }
            let _ = this.update(cx, |this, cx| {
                let id = this.profile_id.clone();
                cx.spawn(async move |this, cx| {
                    let refreshed = cx
                        .background_executor()
                        .spawn(async move { profile_service::get_profile_by_id(&id) })
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        if let Ok(Some(p)) = refreshed {
                            this.state = LoadState::Loaded(p);
                        }
                        cx.notify();
                    });
                })
                .detach();
            });
        })
        .detach();
    }
}

impl Render for LibraryDetailView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let body: AnyElement = match &self.state {
            LoadState::Loading => div().child("Loading…").into_any_element(),
            LoadState::NotFound => div()
                .text_color(rgb(0xef4444))
                .child("Profile not found")
                .into_any_element(),
            LoadState::Failed(e) => div()
                .text_color(rgb(0xef4444))
                .child(format!("Failed: {e}"))
                .into_any_element(),
            LoadState::Loaded(profile) => {
                let bep = profile.bepinex_installed == Some(true);
                let install_btn = (!bep).then(|| {
                    div()
                        .id("install-bepinex")
                        .px_4()
                        .py_2()
                        .rounded_md()
                        .bg(theme.primary)
                        .cursor_pointer()
                        .hover(|s| s.opacity(0.85))
                        .child("Install BepInEx")
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.install_bepinex(cx);
                        }))
                });
                let mod_count = profile.mods.len();
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .child(profile.name.clone()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(profile.path.clone()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(if bep { "BepInEx installed" } else { "BepInEx not installed" }),
                    )
                    .child(div().text_sm().child(format!("{mod_count} mods")))
                    .children(install_btn)
                    .into_any_element()
            }
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_8()
            .pt(px(48.0))
            .child(body)
    }
}
