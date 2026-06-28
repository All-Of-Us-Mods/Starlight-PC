use gpui::*;

use crate::backend::api::{self, ModResponse};
use crate::theme::ThemeExt;
use crate::ui::mod_card::{self, MOD_CARD_HEIGHT};
use gpui_component::Selectable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::pagination::Pagination;
use gpui_component::{Icon, IconName};

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
    /// Total mods for the current query (the API exposes a `/total`
    /// endpoint only for the unfiltered listing). `None` while we
    /// haven't fetched the count yet or when searching (no count
    /// available).
    total: Option<u32>,
    sort: SortBy,
    search_input: Entity<InputState>,
}

enum LoadState {
    Loading,
    Loaded(Vec<ModResponse>),
    Failed(String),
}

impl ExploreView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search mods..."));
        cx.subscribe_in(
            &search_input,
            window,
            |this, state, ev: &InputEvent, _window, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    this.submit_search(state.read(cx).value().to_string(), cx);
                }
            },
        )
        .detach();

        let view = Self {
            state: LoadState::Loading,
            query: String::new(),
            page: 0,
            page_size: 12,
            total: None,
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
                        let total = api::fetch_mods_total().ok();
                        api::fetch_mods(page_size, offset).map(|mods| (mods, total))
                    } else {
                        api::search_mods(&query, page_size, offset).map(|mods| (mods, None))
                    }
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok((mods, total)) => {
                        this.state = LoadState::Loaded(mods);
                        this.total = total;
                    }
                    Err(e) => {
                        this.state = LoadState::Failed(e.to_string());
                    }
                }
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

    fn goto_page(&mut self, one_based_page: usize, cx: &mut Context<Self>) {
        let new_page = (one_based_page.saturating_sub(1)) as u32;
        if new_page == self.page {
            return;
        }
        self.page = new_page;
        self.state = LoadState::Loading;
        cx.notify();
        self.fetch(cx);
    }

    fn sort_pill(
        &self,
        id: &'static str,
        sort: SortBy,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .ghost()
            .selected(self.sort == sort)
            .label(sort.label())
            .on_click(cx.listener(move |this, _, _window, cx| this.set_sort(sort, cx)))
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
        let rows =
            (((middle_height + GRID_GAP) / (MOD_CARD_HEIGHT + GRID_GAP)).floor() as u32).max(1);
        self.update_page_size(columns * rows, cx);

        let body: AnyElement = match &self.state {
            LoadState::Loading => {
                let placeholders: Vec<AnyElement> = (0..(columns * rows))
                    .map(|_| mod_card::mod_card_skeleton(None, &theme).into_any_element())
                    .collect();
                div()
                    .grid()
                    .grid_cols(columns as u16)
                    .gap_4()
                    .flex_1()
                    .overflow_hidden()
                    .children(placeholders)
                    .into_any_element()
            }
            LoadState::Failed(e) => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.danger)
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

        // Prefer the real total from /api/v3/mods/total. When searching
        // the API has no count endpoint, so fall back to "current + 1
        // when this page is full, current otherwise."
        let current = (self.page + 1) as usize;
        let total = match self.total {
            Some(total) => {
                let page_size = self.page_size.max(1) as usize;
                ((total as usize).div_ceil(page_size)).max(1)
            }
            None => {
                let full = matches!(&self.state, LoadState::Loaded(v) if (v.len() as u32) == self.page_size);
                if full { current + 1 } else { current }
            }
        };

        let pagination = (total > 1).then(|| {
            div().flex_none().flex().justify_center().child(
                Pagination::new("explore-pagination")
                    .current_page(current)
                    .total_pages(total)
                    .on_click(cx.listener(|this, page: &usize, _, cx| this.goto_page(*page, cx))),
            )
        });

        let controls = div()
            .flex()
            .items_center()
            .gap_3()
            .flex_none()
            .child(
                div()
                    .flex_1()
                    .child(Input::new(&self.search_input).prefix(Icon::new(IconName::Search))),
            )
            .child(self.sort_pill("sort-downloads", SortBy::Downloads, cx))
            .child(self.sort_pill("sort-updated", SortBy::Updated, cx))
            .child(self.sort_pill("sort-created", SortBy::Created, cx));

        crate::views::page_root("explore-page", &theme)
            .overflow_hidden()
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
            .children(pagination)
    }
}
