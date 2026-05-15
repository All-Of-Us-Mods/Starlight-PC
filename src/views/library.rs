use gpui::*;
use log::warn;

use crate::backend::services::profile_service::{self, ProfileEntry};
use crate::theme::{self, ThemeExt};
use crate::ui::text_input::{TextInput, TextInputEvent};

#[derive(Clone, Debug)]
pub enum LibraryEvent {
    Open(String),
}

impl EventEmitter<LibraryEvent> for LibraryView {}

pub struct LibraryView {
    state: LoadState,
    create_dialog: Option<Entity<TextInput>>,
}

enum LoadState {
    Loading,
    Loaded(Vec<ProfileEntry>),
    Failed(String),
}

impl LibraryView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let view = Self {
            state: LoadState::Loading,
            create_dialog: None,
        };
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { profile_service::get_profiles() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.state = match result {
                    Ok(mut profiles) => {
                        profiles.sort_by_key(|p| std::cmp::Reverse(p.last_launched_at.unwrap_or(0)));
                        LoadState::Loaded(profiles)
                    }
                    Err(e) => {
                        warn!("Failed to load profiles: {e}");
                        LoadState::Failed(e.to_string())
                    }
                };
                cx.notify();
            });
        })
        .detach();
        view
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.state = LoadState::Loading;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { profile_service::get_profiles() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.state = match result {
                    Ok(mut profiles) => {
                        profiles.sort_by_key(|p| std::cmp::Reverse(p.last_launched_at.unwrap_or(0)));
                        LoadState::Loaded(profiles)
                    }
                    Err(e) => LoadState::Failed(e.to_string()),
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn open_create_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| TextInput::new(cx, "Profile name"));
        input.update(cx, |input, cx| input.focus(window, cx));
        cx.subscribe(&input, |this, _, event: &TextInputEvent, cx| match event {
            TextInputEvent::Submit(name) => {
                this.submit_create(name.clone(), cx);
            }
        })
        .detach();
        self.create_dialog = Some(input);
        cx.notify();
    }

    fn submit_create(&mut self, name: String, cx: &mut Context<Self>) {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        self.create_dialog = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::create_profile(&trimmed) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    warn!("Create profile failed: {e}");
                }
                this.refresh(cx);
            });
        })
        .detach();
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .flex()
            .items_center()
            .justify_between()
            .pb_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Library"),
            )
            .child(
                div()
                    .id("create-profile")
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .bg(theme.primary)
                    .text_color(theme.text)
                    .cursor_pointer()
                    .hover(|s| s.opacity(0.85))
                    .child("Create Profile")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.open_create_dialog(window, cx);
                    })),
            )
    }

    fn render_profile_card(
        &self,
        profile: &ProfileEntry,
        theme: &crate::theme::Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = profile.id.clone();
        let emit_id = id.clone();
        div()
            .id(SharedString::from(id))
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .rounded_lg()
            .bg(theme.sidebar_background)
            .border_1()
            .border_color(theme.border)
            .cursor_pointer()
            .hover(|s| s.bg(theme.hover))
            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                cx.emit(LibraryEvent::Open(emit_id.clone()));
            }))
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(profile.name.clone()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child(if profile.bepinex_installed == Some(true) {
                        "BepInEx installed"
                    } else {
                        "BepInEx not installed"
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .child(format!("{} mods", profile.mods.len())),
            )
    }
}

impl Render for LibraryView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let body: AnyElement = match &self.state {
            LoadState::Loading => div()
                .text_color(theme.text_muted)
                .child("Loading profiles…")
                .into_any_element(),
            LoadState::Failed(message) => div()
                .text_color(rgb(0xef4444))
                .child(format!("Failed to load profiles: {message}"))
                .into_any_element(),
            LoadState::Loaded(profiles) if profiles.is_empty() => div()
                .text_color(theme.text_muted)
                .child("No profiles yet. Click \"Create Profile\" to make one.")
                .into_any_element(),
            LoadState::Loaded(profiles) => {
                let cards: Vec<AnyElement> = profiles
                    .iter()
                    .map(|p| self.render_profile_card(p, &theme, cx).into_any_element())
                    .collect();
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_4()
                    .children(cards)
                    .into_any_element()
            }
        };

        let dialog = self.create_dialog.clone().map(|input| {
            let theme = theme.clone();
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
                        .w(px(360.0))
                        .p_5()
                        .rounded_lg()
                        .bg(theme.background)
                        .border_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("New Profile"),
                        )
                        .child(input)
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .justify_end()
                                .child(
                                    div()
                                        .id("cancel-create")
                                        .px_4()
                                        .py_2()
                                        .rounded_md()
                                        .bg(theme.hover)
                                        .cursor_pointer()
                                        .child("Cancel")
                                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                            this.create_dialog = None;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    div()
                                        .id("confirm-create")
                                        .px_4()
                                        .py_2()
                                        .rounded_md()
                                        .bg(theme.primary)
                                        .cursor_pointer()
                                        .child("Create")
                                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                            if let Some(input) = this.create_dialog.clone() {
                                                let name = input.read(cx).value().to_string();
                                                this.submit_create(name, cx);
                                            }
                                        })),
                                ),
                        ),
                )
        });

        div()
            .id("library-page")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .overflow_y_scroll()
            .font_family(theme::FONT_FAMILY)
            .text_color(theme.text)
            .text_size(px(14.0))
            .p_8()
            .pt(px(48.0))
            .child(self.render_header(cx))
            .child(body)
            .children(dialog)
    }
}
