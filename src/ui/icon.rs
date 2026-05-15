use gpui::*;

#[derive(Clone, Copy)]
pub enum IconName {
    Home,
    Compass,
    Library,
    Settings,
    ArrowLeft,
    Plus,
    Play,
    Download,
    Trash,
    Image,
    Starlight,
}

impl IconName {
    fn path(self) -> &'static str {
        match self {
            IconName::Home => "icons/home.svg",
            IconName::Compass => "icons/compass.svg",
            IconName::Library => "icons/library.svg",
            IconName::Settings => "icons/settings.svg",
            IconName::ArrowLeft => "icons/arrow-left.svg",
            IconName::Plus => "icons/plus.svg",
            IconName::Play => "icons/play.svg",
            IconName::Download => "icons/download.svg",
            IconName::Trash => "icons/trash.svg",
            IconName::Image => "icons/image.svg",
            IconName::Starlight => "icons/starlight.svg",
        }
    }
}

/// Render an SVG icon at 16px square in the current text color. Use
/// `icon_sized` for non-default sizes.
pub fn icon(name: IconName) -> Svg {
    icon_sized(name, px(16.0))
}

pub fn icon_sized(name: IconName, size: Pixels) -> Svg {
    svg().path(name.path()).w(size).h(size).flex_none()
}
