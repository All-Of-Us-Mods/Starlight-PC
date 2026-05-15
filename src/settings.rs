use gpui::*;

pub struct AppSettings {
    // fields
}

impl Global for AppSettings {}

pub fn init(cx: &mut App) {
    cx.set_global(AppSettings {
        // init
    });
}
