use gpui::*;

use crate::backend::api::{self, ModResponse, Post};
use crate::theme::{self, ThemeExt};

pub struct HomeView {
    news: Loading<Vec<Post>>,
    trending: Loading<Vec<ModResponse>>,
}

enum Loading<T> {
    Pending,
    Ready(T),
    Failed(String),
}

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

fn news_card(post: &Post, theme: &crate::theme::Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_4()
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
}

fn mod_card(m: &ModResponse, theme: &crate::theme::Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_4()
        .rounded_lg()
        .bg(theme.sidebar_background)
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(m.name.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.text_muted)
                .child(format!("{} downloads", m.downloads)),
        )
}

fn render_loading<T, F>(state: &Loading<T>, render_items: F) -> AnyElement
where
    F: FnOnce(&T) -> AnyElement,
{
    match state {
        Loading::Pending => div().child("Loading…").into_any_element(),
        Loading::Failed(e) => div()
            .text_color(rgb(0xef4444))
            .child(e.clone())
            .into_any_element(),
        Loading::Ready(items) => render_items(items),
    }
}

impl Render for HomeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let news_section = {
            let theme = theme.clone();
            render_loading(&self.news, move |items| {
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(items.iter().take(5).map(|p| news_card(p, &theme).into_any_element()))
                    .into_any_element()
            })
        };
        let trending_section = {
            let theme = theme.clone();
            render_loading(&self.trending, move |items| {
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_3()
                    .children(items.iter().take(6).map(|m| mod_card(m, &theme).into_any_element()))
                    .into_any_element()
            })
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .font_family(theme::FONT_FAMILY)
            .text_color(theme.text)
            .text_size(px(14.0))
            .p_8()
            .pt(px(48.0))
            .gap_8()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Home"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(section_title("News"))
                    .child(news_section),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(section_title("Trending Mods"))
                    .child(trending_section),
            )
    }
}
