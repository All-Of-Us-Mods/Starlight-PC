//! Minimal single-line text input.
//!
//! Not a full editor — no selection, no clipboard, no IME composition,
//! no mid-string cursor positioning. The caret sits at the end of the
//! value. Good enough for naming a profile.

use std::time::Duration;

use gpui::*;

use crate::theme::ThemeExt;

const BLINK_INTERVAL: Duration = Duration::from_millis(530);
const CARET_WIDTH: f32 = 1.5;
const FONT_SIZE: f32 = 14.0;
const CARET_HEIGHT: f32 = 16.0;

pub struct TextInput {
    value: String,
    placeholder: SharedString,
    focus_handle: FocusHandle,
    blink_visible: bool,
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

        // Blink the caret every BLINK_INTERVAL while focused.
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(BLINK_INTERVAL).await;
            if this
                .update(cx, |this, cx| {
                    this.blink_visible = !this.blink_visible;
                    cx.notify();
                })
                .is_err()
            {
                break;
            }
        })
        .detach();

        Self {
            value: String::new(),
            placeholder: placeholder.into(),
            focus_handle,
            blink_visible: true,
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

    fn reset_blink(&mut self) {
        // Keep the caret on briefly after each keystroke so it doesn't
        // disappear mid-type.
        self.blink_visible = true;
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
                if self.value.pop().is_some() {
                    self.reset_blink();
                    cx.notify();
                }
            }
            "enter" => {
                cx.emit(TextInputEvent::Submit(self.value.clone()));
            }
            _ => {
                let modifiers = &event.keystroke.modifiers;
                if modifiers.control || modifiers.platform || modifiers.alt {
                    return;
                }
                if let Some(text) = event.keystroke.key_char.as_ref()
                    && !text.is_empty()
                {
                    self.value.push_str(text);
                    self.reset_blink();
                    cx.notify();
                } else if key.chars().count() == 1 {
                    self.value.push_str(key);
                    self.reset_blink();
                    cx.notify();
                }
            }
        }
    }

    fn caret(theme: &crate::theme::Theme, visible: bool) -> impl IntoElement {
        let color = if visible {
            theme.text
        } else {
            Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }
        };
        div().w(px(CARET_WIDTH)).h(px(CARET_HEIGHT)).bg(color)
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let focused = self.focus_handle.is_focused(window);
        let caret_visible = focused && self.blink_visible;

        let content: AnyElement = if self.value.is_empty() {
            // Placeholder + caret at the start.
            div()
                .flex()
                .items_center()
                .gap_0()
                .child(Self::caret(&theme, caret_visible))
                .child(
                    div()
                        .text_color(theme.text_muted)
                        .child(self.placeholder.clone()),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .gap_0()
                .child(div().text_color(theme.text).child(self.value.clone()))
                .child(Self::caret(&theme, caret_visible))
                .into_any_element()
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
            .text_size(px(FONT_SIZE))
            .child(content)
    }
}
