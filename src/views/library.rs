use gpui::*;
use log::warn;

use crate::backend::events::{self, BackendEvent};
use crate::backend::services::launch_service;
use crate::backend::services::profile_service::{self, ProfileEntry};
use crate::backend::state::game_runtime;
use crate::ui::profile_icon::profile_icon;
use crate::theme::{self, ThemeExt};
use crate::ui::icon::AppIcon;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::skeleton::Skeleton;
use gpui_component::{Disableable, Icon, IconName};

#[derive(Clone, Debug)]
pub enum LibraryEvent {
    Open(String),
}

impl EventEmitter<LibraryEvent> for LibraryView {}

pub struct LibraryView {
    state: LoadState,
    create_dialog: Option<Entity<InputState>>,
    import_dialog: Option<Entity<InputState>>,
    error: Option<String>,
    running_count: usize,
    stoppable_count: usize,
}

enum LoadState {
    Loading,
    Loaded(Vec<ProfileEntry>),
    Failed(String),
}

impl LibraryView {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let initial = game_runtime::current_state();
        let view = Self {
            state: LoadState::Loading,
            create_dialog: None,
            import_dialog: None,
            error: None,
            running_count: initial.running_count,
            stoppable_count: initial.stoppable_running_count,
        };
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { profile_service::get_profiles() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.state = match result {
                    Ok(mut profiles) => {
                        profiles
                            .sort_by_key(|p| std::cmp::Reverse(p.last_launched_at.unwrap_or(0)));
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

        let mut rx = events::subscribe();
        cx.spawn(async move |this, cx| {
            while let Ok(event) = rx.recv().await {
                if let BackendEvent::GameStateChanged(payload) = event {
                    let _ = this.update(cx, |this, cx| {
                        this.running_count = payload.running_count;
                        this.stoppable_count = payload.stoppable_running_count;
                        cx.notify();
                    });
                }
            }
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
                        profiles
                            .sort_by_key(|p| std::cmp::Reverse(p.last_launched_at.unwrap_or(0)));
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
        let state = cx.new(|cx| InputState::new(window, cx).placeholder("Profile name"));
        state.read(cx).focus_handle(cx).focus(window, cx);
        cx.subscribe_in(
            &state,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    this.submit_create(state.read(cx).value().to_string(), cx);
                }
            },
        )
        .detach();
        self.create_dialog = Some(state);
        cx.notify();
    }

    fn open_import_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = cx.new(|cx| InputState::new(window, cx).placeholder("Path to profile .zip"));
        state.read(cx).focus_handle(cx).focus(window, cx);
        cx.subscribe_in(
            &state,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    this.submit_import(state.read(cx).value().to_string(), cx);
                }
            },
        )
        .detach();
        self.import_dialog = Some(state);
        self.error = None;
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

    fn launch_vanilla(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { launch_service::launch_vanilla_from_settings() })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    warn!("vanilla launch failed: {e}");
                    this.error = Some(format!("Vanilla launch failed: {e}"));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn stop_all(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { game_runtime::stop_all_tracked_instances() })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    warn!("stop all failed: {e}");
                    this.error = Some(format!("Stop failed: {e}"));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn submit_import(&mut self, path: String, cx: &mut Context<Self>) {
        let trimmed = path.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        self.import_dialog = None;
        self.error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::import_profile_zip(&trimmed) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    this.error = Some(format!("Import failed: {e}"));
                    cx.notify();
                }
                this.refresh(cx);
            });
        })
        .detach();
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let running = self.running_count;
        let stoppable = self.stoppable_count;

        let launch_or_stop = if running == 0 {
            Button::new("launch-vanilla")
                .icon(Icon::new(IconName::Play))
                .label("Launch Vanilla")
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.launch_vanilla(cx);
                }))
        } else {
            let label = if running > 1 {
                format!("Stop all ({running})")
            } else {
                "Stop".to_string()
            };
            let mut btn = Button::new("stop-all")
                .danger()
                .icon(Icon::new(IconName::Close))
                .label(label);
            if stoppable == 0 {
                // Only UWP instances tracked — can't stop those from here.
                btn = btn.disabled(true);
            } else {
                btn = btn.on_click(cx.listener(|this, _, _window, cx| {
                    this.stop_all(cx);
                }));
            }
            btn
        };

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
                    .flex()
                    .gap_2()
                    .child(launch_or_stop)
                    .child(
                        Button::new("import-profile")
                            .icon(Icon::new(AppIcon::Download))
                            .label("Import Profile")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_import_dialog(window, cx);
                            })),
                    )
                    .child(
                        Button::new("create-profile")
                            .primary()
                            .icon(Icon::new(IconName::Plus))
                            .label("Create Profile")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_create_dialog(window, cx);
                            })),
                    ),
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
            .gap_3()
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
            .child(profile_icon(profile, 48.0, theme))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(profile.name.clone()),
                    )
                    .children((profile.bepinex_installed != Some(true)).then(|| {
                        div()
                            .text_xs()
                            .text_color(rgb(0xf59e0b))
                            .child("BepInEx not installed")
                    }))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(format!("Path: {}", profile.path)),
                    )
                    .child(div().text_xs().text_color(theme.text_muted).child(format!(
                        "{} mods · {}",
                        profile.mods.len(),
                        profile
                            .last_launched_at
                            .map(|_| "played before")
                            .unwrap_or("never launched")
                    ))),
            )
    }
}

fn profile_card_skeleton(theme: &crate::theme::Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .rounded_lg()
        .bg(theme.sidebar_background)
        .border_1()
        .border_color(theme.border)
        .child(Skeleton::new().w_2_3().h_5().rounded_md())
        .child(Skeleton::new().w_1_2().h_4().rounded_md())
        .child(Skeleton::new().w_5_6().h_3().rounded_md())
        .child(Skeleton::new().w_1_3().h_3().rounded_md())
}

impl Render for LibraryView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let body: AnyElement = match &self.state {
            LoadState::Loading => {
                let placeholders: Vec<AnyElement> = (0..4)
                    .map(|_| profile_card_skeleton(&theme).into_any_element())
                    .collect();
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_4()
                    .children(placeholders)
                    .into_any_element()
            }
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

        let create_dialog = self.create_dialog.clone().map(|input| {
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
                        .child(div().font_weight(FontWeight::SEMIBOLD).child("New Profile"))
                        .child(Input::new(&input))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .justify_end()
                                .child(Button::new("cancel-create").label("Cancel").on_click(
                                    cx.listener(|this, _, _window, cx| {
                                        this.create_dialog = None;
                                        cx.notify();
                                    }),
                                ))
                                .child(
                                    Button::new("confirm-create")
                                        .primary()
                                        .label("Create")
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            if let Some(input) = this.create_dialog.clone() {
                                                let name = input.read(cx).value().to_string();
                                                this.submit_create(name, cx);
                                            }
                                        })),
                                ),
                        ),
                )
        });
        let import_dialog = self.import_dialog.clone().map(|input| {
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
                        .w(px(420.0))
                        .p_5()
                        .rounded_lg()
                        .bg(theme.background)
                        .border_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Import Profile ZIP"),
                        )
                        .child(Input::new(&input))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .justify_end()
                                .child(Button::new("cancel-import").label("Cancel").on_click(
                                    cx.listener(|this, _, _window, cx| {
                                        this.import_dialog = None;
                                        cx.notify();
                                    }),
                                ))
                                .child(
                                    Button::new("confirm-import")
                                        .primary()
                                        .label("Import")
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            if let Some(input) = this.import_dialog.clone() {
                                                let path = input.read(cx).value().to_string();
                                                this.submit_import(path, cx);
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
            .children(self.error.clone().map(|message| {
                div()
                    .mb_4()
                    .rounded_md()
                    .bg(rgb(0x7f1d1d))
                    .p_3()
                    .text_color(theme.text)
                    .child(message)
            }))
            .child(body)
            .children(create_dialog)
            .children(import_dialog)
    }
}
