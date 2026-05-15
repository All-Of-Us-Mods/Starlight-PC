use gpui::*;
use crate::theme::ThemeExt;

pub struct Workspace {
    current_view: SharedString,
}

impl Workspace {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            current_view: "Explore".into(),
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        
        div()
            .flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.text)
            .child(
                // Sidebar
                div()
                    .flex()
                    .flex_col()
                    .w(px(250.0))
                    .h_full()
                    .bg(theme.sidebar_background)
                    .border_r_1()
                    .border_color(theme.border)
                    .p_4()
                    .pt_8()
                    .child(
                        div().text_xl().font_weight(FontWeight::BOLD).child("Starlight")
                    )
                    .child(
                        div().mt_8().flex().flex_col().gap_2()
                            .child(self.render_nav_item("Explore", theme.clone(), cx))
                            .child(self.render_nav_item("Library", theme.clone(), cx))
                            .child(self.render_nav_item("Settings", theme.clone(), cx))
                    )
            )
            .child(
                // Main Content
                div()
                    .flex_1()
                    .h_full()
                    .p_8()
                    .child(format!("{} View", self.current_view))
            )
    }
}

impl Workspace {
    fn render_nav_item(&self, name: &'static str, theme: crate::theme::Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active = self.current_view == name;
        div()
            .id(name)
            .px_4()
            .py_2()
            .rounded_md()
            .bg(if is_active { theme.hover } else { rgba(0x00000000) })
            .hover(|style| style.bg(theme.hover))
            .cursor_pointer()
            .child(name)
            .on_click(cx.listener(move |this, _event: &gpui::ClickEvent, _window, _cx| {
                this.current_view = name.into();
            }))
    }
}
