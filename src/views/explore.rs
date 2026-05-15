use gpui::*;

use crate::backend::api::{self, ModResponse};
use crate::theme::{self, ThemeExt};
use crate::ui::mod_card::{self, MOD_CARD_HEIGHT};
use crate::ui::text_input::{TextInput, TextInputEvent};

const MIN_CARD_WIDTH: f32 = 360.0;
const MAX_GRID_COLUMNS: u32 = 4;
const GRID_GAP: f32 = 16.0;
const SIDEBAR_WIDTH: f32 = 220.0;
const PAGE_HORIZONTAL_PADDING: f32 = 64.0;
const PAGE_FIXED_HEIGHT: f32 = 248.0;

#[derive(Clone, Debug)]
pub enum ExploreEvent {
    OpenMod(String),
}

impl EventEmitter<ExploreEvent> for ExploreView {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortBy {
    Downloads,
    Updated,
    Created,
}

impl SortBy {
    fn label(self) -> &'static str {
        match self {
            SortBy::Downloads => "Downloads",
            SortBy::Updated => "Recently Updated",
            SortBy::Created => "Newest",
        }
    }

    fn key(self, m: &ModResponse) -> i64 {
        match self {
            SortBy::Downloads => m.downloads as i64,
            SortBy::Updated => m.updated_at,
            SortBy::Created => m.created_at,
        }
    }
}

pub struct ExploreView {
    state: LoadState,
    query: String,
    page: u32,
    page_size: u32,
    sort: SortBy,
    search_input: Entity<TextInput>,
}

enum LoadState {
    Loading,
    Loaded(Vec<ModResponse>),
    Failed(String),
}

impl ExploreView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| TextInput::new(cx, "Search mods..."));
        cx.subscribe(&search_input, |this, _, ev: &TextInputEvent, cx| match ev {
            TextInputEvent::Submit(q) => this.submit_search(q.clone(), cx),
        })
        .detach();

        let view = Self {
            state: LoadState::Loading,
            query: String::new(),
            page: 0,
            page_size: 12,
            sort: SortBy::Downloads,
            search_input,
        };
        view.fetch(cx);
        view
    }

    fn fetch(&self, cx: &mut Context<Self>) {
        let query = self.query.clone();
        let page_size = self.page_size;
        let offset = self.page * page_size;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if query.is_empty() {
                        api::fetch_mods(page_size, offset)
                    } else {
                        api::search_mods(&query, page_size, offset)
                    }
                })
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
    }

    fn update_page_size(&mut self, page_size: u32, cx: &mut Context<Self>) {
        if page_size == self.page_size {
            return;
        }

        let first_visible_index = self.page * self.page_size;
        self.page_size = page_size;
        self.page = first_visible_index / self.page_size;
        self.state = LoadState::Loading;
        cx.notify();
        self.fetch(cx);
    }

    fn submit_search(&mut self, q: String, cx: &mut Context<Self>) {
        self.query = q.trim().to_string();
        self.page = 0;
        self.state = LoadState::Loading;
        cx.notify();
        self.fetch(cx);
    }

    fn set_sort(&mut self, sort: SortBy, cx: &mut Context<Self>) {
        self.sort = sort;
        cx.notify();
    }

    fn prev_page(&mut self, cx: &mut Context<Self>) {
        if self.page > 0 {
            self.page -= 1;
            self.state = LoadState::Loading;
            cx.notify();
            self.fetch(cx);
        }
    }

    fn next_page(&mut self, cx: &mut Context<Self>) {
        if matches!(&self.state, LoadState::Loaded(v) if v.len() as u32 == self.page_size) {
            self.page += 1;
            self.state = LoadState::Loading;
            cx.notify();
            self.fetch(cx);
        }
    }

    fn sort_pill(
        &self,
        id: &'static str,
        sort: SortBy,
        theme: &crate::theme::Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.sort == sort;
        let text_color = if active { theme.text } else { theme.text_muted };
        div()
            .id(id)
            .px_3()
            .py_1p5()
            .rounded_md()
            .bg(if active {
                theme.hover
            } else {
                theme.sidebar_background
            })
            .border_1()
            .border_color(theme.border)
            .text_color(text_color)
            .cursor_pointer()
            .child(sort.label())
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_sort(sort, cx)))
    }

    fn mod_card(
        m: &ModResponse,
        theme: &crate::theme::Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = SharedString::from(format!("explore-{}", m.id));
        let mod_id_for_click = m.id.clone();
        mod_card::mod_card(id, m, None, theme)
            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                cx.emit(ExploreEvent::OpenMod(mod_id_for_click.clone()));
            }))
            .into_any_element()
    }
}

impl Render for ExploreView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let viewport_size = window.viewport_size();
        let content_width =
            (f32::from(viewport_size.width) - SIDEBAR_WIDTH - PAGE_HORIZONTAL_PADDING)
                .max(MIN_CARD_WIDTH);
        let middle_height =
            (f32::from(viewport_size.height) - PAGE_FIXED_HEIGHT).max(MOD_CARD_HEIGHT);
        let columns = (((content_width + GRID_GAP) / (MIN_CARD_WIDTH + GRID_GAP)).floor() as u32)
            .clamp(1, MAX_GRID_COLUMNS);
        let rows = (((middle_height + GRID_GAP) / (MOD_CARD_HEIGHT + GRID_GAP)).floor() as u32).max(1);
        self.update_page_size(columns * rows, cx);

        let body: AnyElement = match &self.state {
            LoadState::Loading => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text_muted)
                .child("Loading mods…")
                .into_any_element(),
            LoadState::Failed(e) => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(0xef4444))
                .child(format!("Failed to load mods: {e}"))
                .into_any_element(),
            LoadState::Loaded(mods) if mods.is_empty() => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text_muted)
                .child("No mods found.")
                .into_any_element(),
            LoadState::Loaded(mods) => {
                let mut sorted: Vec<&ModResponse> = mods.iter().collect();
                sorted.sort_by_key(|m| std::cmp::Reverse(self.sort.key(m)));
                let cards: Vec<AnyElement> = sorted
                    .iter()
                    .map(|m| Self::mod_card(m, &theme, cx))
                    .collect();
                div()
                    .grid()
                    .grid_cols(columns as u16)
                    .gap_4()
                    .flex_1()
                    .overflow_hidden()
                    .children(cards)
                    .into_any_element()
            }
        };

        let can_prev = self.page > 0;
        let can_next = matches!(&self.state, LoadState::Loaded(v) if v.len() as u32 == self.page_size);

        let pagination = div()
            .flex()
            .items_center()
            .justify_between()
            .flex_none()
            .child(
                div()
                    .id("prev")
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(if can_prev {
                        theme.hover
                    } else {
                        theme.sidebar_background
                    })
                    .text_color(if can_prev {
                        theme.text
                    } else {
                        theme.text_muted
                    })
                    .border_1()
                    .border_color(theme.border)
                    .cursor_pointer()
                    .child("← Prev")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.prev_page(cx))),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child(format!("Page {}", self.page + 1)),
            )
            .child(
                div()
                    .id("next")
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(if can_next {
                        theme.hover
                    } else {
                        theme.sidebar_background
                    })
                    .text_color(if can_next {
                        theme.text
                    } else {
                        theme.text_muted
                    })
                    .border_1()
                    .border_color(theme.border)
                    .cursor_pointer()
                    .child("Next →")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.next_page(cx))),
            );

        let controls = div()
            .flex()
            .items_center()
            .gap_3()
            .flex_none()
            .child(div().flex_1().child(self.search_input.clone()))
            .child(self.sort_pill("sort-downloads", SortBy::Downloads, &theme, cx))
            .child(self.sort_pill("sort-updated", SortBy::Updated, &theme, cx))
            .child(self.sort_pill("sort-created", SortBy::Created, &theme, cx));

        div()
            .id("explore-page")
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .font_family(theme::FONT_FAMILY)
            .text_color(theme.text)
            .text_size(px(14.0))
            .p_8()
            .pt(px(48.0))
            .gap_4()
            .child(
                div()
                    .flex_none()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Explore"),
            )
            .child(controls)
            .child(div().flex_1().overflow_hidden().child(body))
            .child(pagination)
    }
}
