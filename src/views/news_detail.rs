use chrono::{DateTime, Local};
use gpui::*;

use crate::backend::api::Post;
use crate::theme::ThemeExt;
use crate::views::{page_root, section_label};

pub struct NewsDetailView {
    post: Post,
}

impl NewsDetailView {
    pub fn new(post: Post) -> Self {
        Self { post }
    }

    /// Title shown in the app title bar — the post headline.
    pub fn title(&self) -> SharedString {
        self.post.title.clone().into()
    }
}

fn format_date(timestamp_ms: i64) -> String {
    DateTime::from_timestamp_millis(timestamp_ms)
        .map(|date| date.with_timezone(&Local).format("%B %-d, %Y").to_string())
        .unwrap_or_else(|| "Unknown date".to_string())
}

impl Render for NewsDetailView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        page_root("news-detail-page", &theme)
            .overflow_y_scroll()
            .gap_6()
            .child(
                div()
                    .max_w(px(840.0))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_sm()
                            .text_color(theme.primary)
                            .child(format_date(self.post.updated_at)),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .child(self.post.title.clone()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(format!("Posted by {}", self.post.author)),
                    )
                    .child(div().h(px(1.0)).w_full().bg(theme.border))
                    .child(section_label("Content", &theme))
                    .child(
                        div()
                            .text_sm()
                            .line_height(px(22.0))
                            .text_color(theme.text)
                            .child(self.post.content.clone()),
                    ),
            )
    }
}
