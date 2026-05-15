use gpui::*;

use crate::backend::api::{self, ModResponse};
use crate::theme::{self, ThemeExt};

#[derive(Clone, Debug)]
pub enum ExploreEvent {
    OpenMod(String),
}

impl EventEmitter<ExploreEvent> for ExploreView {}

pub struct ExploreView {
    state: LoadState,
}

enum LoadState {
    Loading,
    Loaded(Vec<ModResponse>),
    Failed(String),
}

impl ExploreView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let view = Self {
            state: LoadState::Loading,
        };
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async { api::fetch_mods(50, 0) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.state = match result {
                    Ok(v) => LoadState::Loaded(v),
                    Err(e) => LoadState::Failed(e.to_string()),
                };
                cx.notify();
            });
        })
        .detach();
        view
    }

    fn mod_card(m: &ModResponse, theme: &crate::theme::Theme, cx: &mut Context<Self>) -> AnyElement {
        let id = SharedString::from(format!("explore-{}", m.id));
        let mod_id_for_click = m.id.clone();
        div()
            .id(id)
            .flex()
            .flex_col()
            .rounded_lg()
            .overflow_hidden()
            .bg(theme.sidebar_background)
            .border_1()
            .border_color(theme.border)
            .cursor_pointer()
            .hover(|s| s.border_color(theme.primary))
            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                cx.emit(ExploreEvent::OpenMod(mod_id_for_click.clone()));
            }))
            .child(
                img(api::mod_thumbnail_url(&m.id))
                    .w_full()
                    .h(px(160.0))
                    .object_fit(ObjectFit::Cover)
                    .bg(theme.hover),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_4()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(m.name.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(m.description.chars().take(120).collect::<String>()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(format!("{} · {} downloads", m.author, m.downloads)),
                    ),
            )
            .into_any_element()
    }
}

impl Render for ExploreView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let body: AnyElement = match &self.state {
            LoadState::Loading => div()
                .text_color(theme.text_muted)
                .child("Loading mods…")
                .into_any_element(),
            LoadState::Failed(e) => div()
                .text_color(rgb(0xef4444))
                .child(format!("Failed to load mods: {e}"))
                .into_any_element(),
            LoadState::Loaded(mods) if mods.is_empty() => div()
                .text_color(theme.text_muted)
                .child("No mods found.")
                .into_any_element(),
            LoadState::Loaded(mods) => {
                let cards: Vec<AnyElement> = mods
                    .iter()
                    .map(|m| Self::mod_card(m, &theme, cx))
                    .collect();
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_4()
                    .children(cards)
                    .into_any_element()
            }
        };

        div()
            .id("explore-page")
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
                    .child("Explore"),
            )
            .child(body)
    }
}
