use gpui::*;

use crate::backend::api::{self, ModResponse};
use crate::theme::{self, ThemeExt};
use crate::ui::icon::{icon, IconName};

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
    Loaded(ModResponse),
    Failed(String),
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
                .spawn(async move { api::fetch_mod(&id) })
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

impl Render for ModDetailView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let back = div()
            .id("back")
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(theme.hover)
            .cursor_pointer()
            .hover(|s| s.opacity(0.85))
            .child(icon(IconName::ArrowLeft))
            .child("Back")
            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                cx.emit(ModDetailEvent::Close);
            }));

        let body: AnyElement = match &self.state {
            LoadState::Loading => div()
                .text_color(theme.text_muted)
                .child("Loading…")
                .into_any_element(),
            LoadState::Failed(e) => div()
                .text_color(rgb(0xef4444))
                .child(format!("Failed: {e}"))
                .into_any_element(),
            LoadState::Loaded(m) => div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    img(api::mod_thumbnail_url(&m.id))
                        .w_full()
                        .h(px(280.0))
                        .object_fit(ObjectFit::Cover)
                        .rounded_lg()
                        .bg(theme.hover),
                )
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::BOLD)
                        .child(m.name.clone()),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .text_sm()
                        .text_color(theme.text_muted)
                        .child(format!("by {}", m.author))
                        .child(format!("{} downloads", m.downloads))
                        .children(m.mod_type.clone().map(|t| t)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.text)
                        .child(m.description.clone()),
                )
                .into_any_element(),
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
