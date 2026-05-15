//! Minimal single-line text input.
//!
//! Not a full editor — no selection, no clipboard, no IME composition.
//! It captures key presses on the focused entity and appends or removes
//! one character at a time. Good enough for naming a profile.

use gpui::*;

use crate::theme::ThemeExt;

pub struct TextInput {
    value: String,
    placeholder: SharedString,
    focus_handle: FocusHandle,
}

#[derive(Clone, Debug)]
pub enum TextInputEvent {
    Submit(String),
}

impl EventEmitter<TextInputEvent> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>, placeholder: impl Into<SharedString>) -> Self {
        let focus_handle = cx.focus_handle();
        Self {
            value: String::new(),
            placeholder: placeholder.into(),
            focus_handle,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.value.clear();
        cx.notify();
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus_handle.focus(window, cx);
    }

    fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        match key {
            "backspace" => {
                self.value.pop();
                cx.notify();
            }
            "enter" => {
                cx.emit(TextInputEvent::Submit(self.value.clone()));
            }
            _ => {
                // Skip modifier-only / non-typing combos. key_char
                // carries the actual typed character (handles shifted
                // chars, dead keys, etc.).
                let modifiers = &event.keystroke.modifiers;
                if modifiers.control || modifiers.platform || modifiers.alt {
                    return;
                }
                if let Some(text) = event.keystroke.key_char.as_ref()
                    && !text.is_empty()
                {
                    self.value.push_str(text);
                    cx.notify();
                } else if key.chars().count() == 1 {
                    self.value.push_str(key);
                    cx.notify();
                }
            }
        }
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let focused = self.focus_handle.is_focused(window);
        let display: SharedString = if self.value.is_empty() {
            self.placeholder.clone()
        } else {
            self.value.clone().into()
        };
        let text_color = if self.value.is_empty() {
            theme.text_muted
        } else {
            theme.text
        };

        div()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
            .flex()
            .items_center()
            .h(px(36.0))
            .px_3()
            .rounded_md()
            .bg(theme.sidebar_background)
            .border_1()
            .border_color(if focused { theme.primary } else { theme.border })
            .text_color(text_color)
            .child(display)
            .child(
                // simple blinking-less caret
                div()
                    .ml_1()
                    .w(px(1.0))
                    .h(px(16.0))
                    .bg(if focused {
                        theme.text
                    } else {
                        Rgba {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }
                    }),
            )
    }
}
