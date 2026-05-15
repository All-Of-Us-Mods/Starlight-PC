use gpui::*;

use crate::backend::api::{self, ModResponse};
use crate::theme::{self, ThemeExt};
use crate::ui::icon::{icon, IconName};
use crate::ui::text_input::{TextInput, TextInputEvent};

const PAGE_SIZE: u32 = 12;

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
            sort: SortBy::Downloads,
            search_input,
        };
        view.fetch(cx);
        view
    }

    fn fetch(&self, cx: &mut Context<Self>) {
        let query = self.query.clone();
        let offset = self.page * PAGE_SIZE;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if query.is_empty() {
                        api::fetch_mods(PAGE_SIZE, offset)
                    } else {
                        api::search_mods(&query, PAGE_SIZE, offset)
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
        if matches!(&self.state, LoadState::Loaded(v) if v.len() as u32 == PAGE_SIZE) {
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
            .bg(if active { theme.hover } else { theme.sidebar_background })
            .border_1()
            .border_color(theme.border)
            .text_color(text_color)
            .cursor_pointer()
            .child(sort.label())
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_sort(sort, cx)))
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
                let mut sorted: Vec<&ModResponse> = mods.iter().collect();
                sorted.sort_by_key(|m| std::cmp::Reverse(self.sort.key(m)));
                let cards: Vec<AnyElement> = sorted
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

        let can_prev = self.page > 0;
        let can_next = matches!(&self.state, LoadState::Loaded(v) if v.len() as u32 == PAGE_SIZE);

        let pagination = div()
            .flex()
            .items_center()
            .justify_between()
            .pt_4()
            .child(
                div()
                    .id("prev")
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(if can_prev { theme.hover } else { theme.sidebar_background })
                    .text_color(if can_prev { theme.text } else { theme.text_muted })
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
                    .bg(if can_next { theme.hover } else { theme.sidebar_background })
                    .text_color(if can_next { theme.text } else { theme.text_muted })
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
            .child(div().flex_1().child(self.search_input.clone()))
            .child(self.sort_pill("sort-downloads", SortBy::Downloads, &theme, cx))
            .child(self.sort_pill("sort-updated", SortBy::Updated, &theme, cx))
            .child(self.sort_pill("sort-created", SortBy::Created, &theme, cx));

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
            .child(controls)
            .child(body)
            .child(pagination)
    }
}
