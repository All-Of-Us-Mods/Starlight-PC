use chrono::{DateTime, Local};
use gpui::*;

use crate::backend::api::Post;
use crate::theme::{self, ThemeExt};
use crate::views::section_label;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{Icon, IconName};

#[derive(Clone, Debug)]
pub enum NewsDetailEvent {
    Close,
}

impl EventEmitter<NewsDetailEvent> for NewsDetailView {}

pub struct NewsDetailView {
    post: Post,
}

impl NewsDetailView {
    pub fn new(post: Post) -> Self {
        Self { post }
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

        let back = Button::new("back")
            .ghost()
            .icon(Icon::new(IconName::ArrowLeft))
            .label("Back")
            .on_click(cx.listener(|_, _, _window, cx| {
                cx.emit(NewsDetailEvent::Close);
            }));

        div()
            .id("news-detail-page")
            .flex()
            .flex_col()
            .size_full()
            .overflow_y_scroll()
            .font_family(theme::FONT_FAMILY)
            .text_color(theme.text)
            .text_size(px(14.0))
            .p_8()
            .pt(px(48.0))
            .gap_6()
            .child(back)
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
