//! Profile icon picker dialog. Lives as part of the library-detail page —
//! state hangs off [`LibraryDetailView`], and the overlay only renders while
//! `icon_dialog` is `Some`.

use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{Icon, IconName};

use super::{LibraryDetailView, LoadState};
use crate::backend::api;
use crate::backend::services::profile_service::{self, ProfileEntry, ProfileIconSelection};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IconDialogMode {
    Default,
    Custom,
    Mod,
}

pub struct IconDialogState {
    pub mode: IconDialogMode,
    pub selected_mod_id: Option<String>,
    pub pending_custom: Option<(Vec<u8>, String)>,
    pub error: Option<String>,
}

impl LibraryDetailView {
    pub(super) fn open_icon_dialog(&mut self, cx: &mut Context<Self>) {
        let LoadState::Loaded(profile) = &self.state else {
            return;
        };
        let mode = match profile.icon_mode.as_deref() {
            Some("custom") => IconDialogMode::Custom,
            Some("mod") => IconDialogMode::Mod,
            _ => IconDialogMode::Default,
        };
        let selected_mod_id = profile
            .icon_mod_id
            .clone()
            .or_else(|| profile.mods.first().map(|m| m.mod_id.clone()));
        self.icon_dialog = Some(IconDialogState {
            mode,
            selected_mod_id,
            pending_custom: None,
            error: None,
        });
        cx.notify();
    }

    pub(super) fn set_icon_mode(&mut self, mode: IconDialogMode, cx: &mut Context<Self>) {
        if let Some(state) = self.icon_dialog.as_mut() {
            state.mode = mode;
            state.error = None;
            if mode == IconDialogMode::Mod && state.selected_mod_id.is_none() {
                if let LoadState::Loaded(profile) = &self.state {
                    state.selected_mod_id = profile.mods.first().map(|m| m.mod_id.clone());
                }
            }
            cx.notify();
        }
    }

    pub(super) fn pick_custom_icon(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose icon image".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| format!(".{}", s.to_lowercase()))
                .unwrap_or_default();
            let read = cx
                .background_executor()
                .spawn(async move { std::fs::read(&path) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Some(state) = this.icon_dialog.as_mut() {
                    match read {
                        Ok(bytes) if !bytes.is_empty() => {
                            state.pending_custom = Some((bytes, extension));
                            state.error = None;
                        }
                        Ok(_) => state.error = Some("Selected image is empty".into()),
                        Err(e) => state.error = Some(format!("Failed to read image: {e}")),
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(super) fn save_icon(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.icon_dialog.as_ref() else {
            return;
        };
        let selection = match state.mode {
            IconDialogMode::Default => ProfileIconSelection::Default,
            IconDialogMode::Custom => {
                let LoadState::Loaded(profile) = &self.state else {
                    return;
                };
                let has_existing = profile.icon_mode.as_deref() == Some("custom")
                    && profile.custom_icon_extension.is_some();
                match state.pending_custom.clone() {
                    Some((bytes, extension)) => ProfileIconSelection::Custom { bytes, extension },
                    None if has_existing => {
                        self.icon_dialog = None;
                        cx.notify();
                        return;
                    }
                    None => {
                        if let Some(s) = self.icon_dialog.as_mut() {
                            s.error = Some("Choose an image for the custom icon".into());
                        }
                        cx.notify();
                        return;
                    }
                }
            }
            IconDialogMode::Mod => {
                let Some(mod_id) = state.selected_mod_id.clone() else {
                    if let Some(s) = self.icon_dialog.as_mut() {
                        s.error = Some("Select an installed mod icon".into());
                    }
                    cx.notify();
                    return;
                };
                ProfileIconSelection::Mod { mod_id }
            }
        };

        let id = self.profile_id.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::update_profile_icon(&id, selection) })
                .await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(()) => {
                    this.icon_dialog = None;
                    this.spawn_load(cx);
                }
                Err(e) => {
                    if let Some(s) = this.icon_dialog.as_mut() {
                        s.error = Some(format!("Failed to update icon: {e}"));
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

pub(super) fn render_icon_dialog(
    state: &IconDialogState,
    profile: &ProfileEntry,
    theme: crate::theme::Theme,
    cx: &mut Context<LibraryDetailView>,
) -> AnyElement {
    let mode = state.mode;
    let mode_button = |id: &'static str, label: &'static str, target: IconDialogMode| {
        let mut btn = Button::new(id).label(label).on_click(
            cx.listener(move |this, _: &ClickEvent, _, cx| this.set_icon_mode(target, cx)),
        );
        if mode == target {
            btn = btn.primary();
        }
        btn
    };

    let mode_row = div()
        .flex()
        .gap_2()
        .child(mode_button(
            "icon-mode-default",
            "Default",
            IconDialogMode::Default,
        ))
        .child(mode_button(
            "icon-mode-custom",
            "Custom Image",
            IconDialogMode::Custom,
        ))
        .child(mode_button(
            "icon-mode-mod",
            "Installed Mod",
            IconDialogMode::Mod,
        ));

    let body: AnyElement = match mode {
        IconDialogMode::Default => div()
            .text_sm()
            .text_color(theme.text_muted)
            .child("Use the default profile icon.")
            .into_any_element(),
        IconDialogMode::Custom => {
            let has_pending = state.pending_custom.is_some();
            let has_existing = profile.icon_mode.as_deref() == Some("custom")
                && profile.custom_icon_extension.is_some();
            let status: AnyElement = if has_pending {
                div()
                    .text_sm()
                    .child("New image ready to save.")
                    .into_any_element()
            } else if has_existing {
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child("Using existing custom image. Choose a new one to replace it.")
                    .into_any_element()
            } else {
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child("PNG, JPG, WEBP, GIF, BMP, or AVIF.")
                    .into_any_element()
            };
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    Button::new("icon-pick-file")
                        .icon(Icon::new(IconName::FolderOpen))
                        .label(if has_pending || has_existing {
                            "Change Image"
                        } else {
                            "Choose Image"
                        })
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.pick_custom_icon(window, cx);
                        })),
                )
                .child(status)
                .into_any_element()
        }
        IconDialogMode::Mod => {
            let mods: Vec<String> = profile
                .mods
                .iter()
                .map(|m| m.mod_id.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            if mods.is_empty() {
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child("No mods installed. Add a mod to use its icon.")
                    .into_any_element()
            } else {
                let selected = state.selected_mod_id.clone();
                let theme_for_items = theme.clone();
                let items: Vec<AnyElement> = mods
                    .into_iter()
                    .map(|mod_id| {
                        let is_selected = selected.as_deref() == Some(mod_id.as_str());
                        let click_id = mod_id.clone();
                        div()
                            .id(SharedString::from(format!("icon-mod-{mod_id}")))
                            .flex()
                            .items_center()
                            .gap_2()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(if is_selected {
                                theme_for_items.primary
                            } else {
                                theme_for_items.border
                            })
                            .cursor_pointer()
                            .hover(|s| s.bg(theme_for_items.hover))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                if let Some(s) = this.icon_dialog.as_mut() {
                                    s.selected_mod_id = Some(click_id.clone());
                                    s.error = None;
                                    cx.notify();
                                }
                            }))
                            .child(
                                img(api::mod_thumbnail_url(&mod_id))
                                    .w(px(36.0))
                                    .h(px(36.0))
                                    .rounded_md()
                                    .object_fit(ObjectFit::Cover),
                            )
                            .child(div().text_sm().truncate().child(mod_id))
                            .into_any_element()
                    })
                    .collect();
                div()
                    .id("icon-mod-list")
                    .max_h(px(240.0))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(items)
                    .into_any_element()
            }
        }
    };

    let error_row = state
        .error
        .clone()
        .map(|msg| div().text_sm().text_color(rgb(0xef4444)).child(msg));

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
                .w(px(480.0))
                .p_5()
                .rounded_lg()
                .bg(theme.background)
                .border_1()
                .border_color(theme.border)
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Edit Profile Icon"),
                )
                .child(mode_row)
                .child(body)
                .children(error_row)
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .justify_end()
                        .child(Button::new("icon-dialog-cancel").label("Cancel").on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.icon_dialog = None;
                                cx.notify();
                            }),
                        ))
                        .child(
                            Button::new("icon-dialog-save")
                                .primary()
                                .label("Save")
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.save_icon(cx);
                                })),
                        ),
                ),
        )
        .into_any_element()
}
