use gpui::*;

#[derive(Clone)]
pub struct Theme {
    pub background: Rgba,
    pub sidebar_background: Rgba,
    pub primary: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub border: Rgba,
    pub hover: Rgba,
}

impl Global for Theme {}

pub fn init(cx: &mut App) {
    cx.set_global(Theme {
        background: rgb(0x09090b),
        sidebar_background: rgb(0x18181b),
        primary: rgb(0x3b82f6),
        text: rgb(0xfafafa),
        text_muted: rgb(0xa1a1aa),
        border: rgb(0x27272a),
        hover: rgb(0x27272a),
    });
}

pub const FONT_FAMILY: &str = ".SystemUIFont";

pub trait ThemeExt {
    fn theme(&self) -> &Theme;
}

impl<'a, V> ThemeExt for Context<'a, V> {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}

impl ThemeExt for App {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}
