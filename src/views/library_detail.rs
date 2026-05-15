use gpui::*;
use log::warn;

use crate::backend::events::{self, BackendEvent};
use crate::backend::services::bepinex_service::{BepInExProgress, BepInExTargetType};
use crate::backend::services::profile_service::{self, ProfileEntry};
use crate::theme::{self, ThemeExt};

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
            bep_progress: None,
            confirming_delete: false,
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

    fn button(
        id: &'static str,
        label: SharedString,
        bg: Rgba,
        theme: &crate::theme::Theme,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .px_4()
            .py_2()
            .rounded_md()
            .bg(bg)
            .text_color(theme.text)
            .cursor_pointer()
            .hover(|s| s.opacity(0.85))
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| on_click(this, cx)))
    }
}

impl Render for LibraryDetailView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let back = Self::button(
            "back",
            "← Back".into(),
            theme.hover,
            &theme,
            |_, cx| cx.emit(LibraryDetailEvent::Close),
            cx,
        );

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
                let bep_installed = profile.bepinex_installed == Some(true);
                let installing = self.bep_progress.is_some();

                let install_btn = (!bep_installed && !installing).then(|| {
                    Self::button(
                        "install-bepinex",
                        "Install BepInEx".into(),
                        theme.primary,
                        &theme,
                        |this, cx| this.install_bepinex(cx),
                        cx,
                    )
                });

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
                        .child(
                            div()
                                .w_full()
                                .h(px(6.0))
                                .rounded_full()
                                .bg(theme.hover)
                                .child(
                                    div()
                                        .h_full()
                                        .w(relative((p.progress as f32 / 100.0).clamp(0.0, 1.0)))
                                        .rounded_full()
                                        .bg(theme.primary),
                                ),
                        )
                });

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
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .text_color(theme.text_muted)
                                .child("Delete this profile?"),
                        )
                        .child(Self::button(
                            "confirm-delete",
                            "Delete".into(),
                            rgb(0xdc2626),
                            &theme,
                            |this, cx| this.delete_profile(cx),
                            cx,
                        ))
                        .child(Self::button(
                            "cancel-delete",
                            "Cancel".into(),
                            theme.hover,
                            &theme,
                            |this, cx| {
                                this.confirming_delete = false;
                                cx.notify();
                            },
                            cx,
                        ))
                } else {
                    div().child(Self::button(
                        "delete-profile",
                        "Delete Profile".into(),
                        rgb(0xdc2626),
                        &theme,
                        |this, cx| {
                            this.confirming_delete = true;
                            cx.notify();
                        },
                        cx,
                    ))
                };

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
                        div().text_sm().child(if bep_installed {
                            "BepInEx installed"
                        } else {
                            "BepInEx not installed"
                        }),
                    )
                    .children(progress_row)
                    .children(install_btn)
                    .children(mods_section)
                    .child(delete_row)
                    .into_any_element()
            }
        };

        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .font_family(theme::FONT_FAMILY)
            .text_color(theme.text)
            .text_size(px(14.0))
            .p_8()
            .pt(px(48.0))
            .child(back)
            .child(body)
    }
}
