use gpui::*;

use crate::backend::api::{self, ModResponse, Post};
use crate::theme::{self, ThemeExt};
use crate::ui::mod_card;

#[derive(Clone, Debug)]
pub enum HomeEvent {
    OpenMod(String),
}

impl EventEmitter<HomeEvent> for HomeView {}

pub struct HomeView {
    news: Loading<Vec<Post>>,
    trending: Loading<Vec<ModResponse>>,
}

enum Loading<T> {
    Pending,
    Ready(T),
    Failed(String),
}

const CARD_WIDTH: f32 = 420.0;
const NEWS_CARD_WIDTH: f32 = 320.0;

impl HomeView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let view = Self {
            news: Loading::Pending,
            trending: Loading::Pending,
        };
        cx.spawn(async move |this, cx| {
            let news = cx
                .background_executor()
                .spawn(async { api::fetch_news() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.news = match news {
                    Ok(v) => Loading::Ready(v),
                    Err(e) => Loading::Failed(e.to_string()),
                };
                cx.notify();
            });
            let trending = cx
                .background_executor()
                .spawn(async { api::fetch_trending_mods() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.trending = match trending {
                    Ok(v) => Loading::Ready(v),
                    Err(e) => Loading::Failed(e.to_string()),
                };
                cx.notify();
            });
        })
        .detach();
        view
    }
}

fn section_title(text: &'static str) -> impl IntoElement {
    div()
        .text_lg()
        .font_weight(FontWeight::SEMIBOLD)
        .pb_3()
        .child(text)
}

fn carousel(id: &'static str, items: Vec<AnyElement>) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .gap_3()
        .overflow_x_scroll()
        .pb_2()
        .children(items)
}

fn news_card(post: &Post, theme: &crate::theme::Theme) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .w(px(NEWS_CARD_WIDTH))
        .flex_shrink_0()
        .rounded_lg()
        .bg(theme.sidebar_background)
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(post.title.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.text_muted)
                .child(format!("by {}", post.author)),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.text_muted)
                .child(post.content.chars().take(140).collect::<String>()),
        )
        .into_any_element()
}

impl HomeView {
    fn mod_card(
        m: &ModResponse,
        theme: &crate::theme::Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = SharedString::from(format!("trending-{}", m.id));
        let mod_id_for_click = m.id.clone();
        mod_card::mod_card(id, m, Some(px(CARD_WIDTH)), theme)
            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                cx.emit(HomeEvent::OpenMod(mod_id_for_click.clone()));
            }))
            .into_any_element()
    }
}

impl Render for HomeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let news_body: AnyElement = match &self.news {
            Loading::Pending => div()
                .text_color(theme.text_muted)
                .child("Loading news…")
                .into_any_element(),
            Loading::Failed(e) => div()
                .text_color(rgb(0xef4444))
                .child(e.clone())
                .into_any_element(),
            Loading::Ready(items) => carousel(
                "news-carousel",
                items.iter().map(|p| news_card(p, &theme)).collect(),
            )
            .into_any_element(),
        };

        let trending_body: AnyElement = match &self.trending {
            Loading::Pending => div()
                .text_color(theme.text_muted)
                .child("Loading mods…")
                .into_any_element(),
            Loading::Failed(e) => div()
                .text_color(rgb(0xef4444))
                .child(e.clone())
                .into_any_element(),
            Loading::Ready(items) => carousel(
                "trending-carousel",
                items
                    .iter()
                    .map(|m| Self::mod_card(m, &theme, cx))
                    .collect(),
            )
            .into_any_element(),
        };

        div()
            .id("home-page")
            .flex()
            .flex_col()
            .size_full()
            .overflow_y_scroll()
            .font_family(theme::FONT_FAMILY)
            .text_color(theme.text)
            .text_size(px(14.0))
            .p_8()
            .pt(px(48.0))
            .gap_8()
            .child(div().text_2xl().font_weight(FontWeight::BOLD).child("Home"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(section_title("News"))
                    .child(news_body),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(section_title("Trending Mods"))
                    .child(trending_body),
            )
    }
}
