//! App-specific icons that have no built-in equivalent in gpui-component.
//!
//! Use `gpui_component::IconName` first; only reach for `AppIcon` when no
//! built-in icon fits (sidebar nav glyphs, the app logo, etc.). Both enums
//! implement [`gpui_component::IconNamed`], so they're interchangeable in
//! any API that takes `impl Into<Icon>`.

use gpui::SharedString;
use gpui_component::IconNamed;

#[derive(Clone, Copy)]
pub enum AppIcon {
    Home,
    Compass,
    Library,
    Download,
    Starlight,
}

impl IconNamed for AppIcon {
    fn path(self) -> SharedString {
        match self {
            AppIcon::Home => "icons/home.svg".into(),
            AppIcon::Compass => "icons/compass.svg".into(),
            AppIcon::Library => "icons/library.svg".into(),
            AppIcon::Download => "icons/download.svg".into(),
            AppIcon::Starlight => "icons/starlight.svg".into(),
        }
    }
}
